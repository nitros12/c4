use std::convert::TryInto;
use std::time::Duration;

use dialoguer;
use num_enum::{IntoPrimitive, TryFromPrimitive};
use rubot;

const BOARD_HEIGHT: usize = 6;
const BOARD_WIDTH: usize = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Colour {
    Red,
    Yellow,
}

impl Colour {
    fn invert(self) -> Self {
        match self {
            Colour::Red => Colour::Yellow,
            Colour::Yellow => Colour::Red,
        }
    }
}

impl std::fmt::Display for Colour {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let col = match self {
            Colour::Red => "R",
            Colour::Yellow => "Y",
        };

        write!(f, "{}", col)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Winner {
    Red,
    Yellow,
    Tie,
}

impl Winner {
    fn from_colour(colour: Colour) -> Self {
        match colour {
            Colour::Red => Winner::Red,
            Colour::Yellow => Winner::Yellow,
        }
    }

    fn to_colour(self) -> Option<Colour> {
        match self {
            Winner::Red => Some(Colour::Red),
            Winner::Yellow => Some(Colour::Yellow),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum Column {
    A = 0,
    B,
    C,
    D,
    E,
    F,
}

impl std::fmt::Display for Column {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let col = match self {
            Column::A => "A",
            Column::B => "B",
            Column::C => "C",
            Column::D => "D",
            Column::E => "E",
            Column::F => "F",
        };

        write!(f, "{}", col)
    }
}

impl Column {
    fn to_idx(self) -> usize {
        u8::from(self) as usize
    }

    fn offset(self, offset: i8) -> Option<Column> {
        let v = u8::from(self) as i16 + offset as i16;
        (v as u8).try_into().ok()
    }

    // fn succ(self) -> Option<Column> {
    //     self.offset(1)
    // }

    // fn pred(self) -> Option<Column> {
    //     self.offset(-1)
    // }

    fn all() -> &'static [Column] {
        const ALL: &'static [Column] = &[
            Column::A,
            Column::B,
            Column::C,
            Column::D,
            Column::E,
            Column::F,
        ];

        ALL
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Board {
    heights: [u8; BOARD_WIDTH],
    tiles: [[Option<Colour>; BOARD_HEIGHT]; BOARD_WIDTH],
}

impl Board {
    fn new() -> Self {
        Self {
            heights: Default::default(),
            tiles: Default::default(),
        }
    }

    fn column_height(&self, column: Column) -> u8 {
        self.heights[column.to_idx()]
    }

    fn column_full(&self, column: Column) -> bool {
        self.column_height(column) >= BOARD_HEIGHT as u8
    }

    fn place_on_column(&mut self, column: Column, colour: Colour) {
        self.tiles[column.to_idx()][self.column_height(column) as usize] = Some(colour);
        self.heights[column.to_idx()] += 1;
    }

    fn piece_at(&self, column: Column, height: u8) -> Option<Colour> {
        self.tiles[column.to_idx()][height as usize]
    }

    fn allowed_columns(&self) -> Vec<Column> {
        Column::all()
            .iter()
            .cloned()
            .filter(|c| !self.column_full(*c))
            .collect()
    }

    fn render(&self) {
        for i in (0..BOARD_HEIGHT).rev() {
            for &col in Column::all() {
                match self.piece_at(col, i as u8) {
                    Some(p) => print!("{}", p),
                    None => print!("_"),
                };
            }

            println!("");
        }
    }
}

fn row_offset(row: u8, offset: i8) -> Option<u8> {
    let v = row as i16 + offset as i16;
    let h = BOARD_HEIGHT as i16;
    if v < 0 || v >= h {
        None
    } else {
        Some(v as u8)
    }
}

#[derive(Debug, Clone)]
enum MoveError {
    GameOver,
    ColumnFull(Column),
}

#[derive(Debug, Clone)]
struct Game {
    state: Board,
    current_colour: Colour,
    winner: Option<Winner>,
}

impl Game {
    fn new(starting_colour: Colour) -> Self {
        Self {
            state: Board::new(),
            current_colour: starting_colour,
            winner: None,
        }
    }

    fn make_move(&mut self, column: Column) -> Result<(), MoveError> {
        if self.is_finished() {
            return Err(MoveError::GameOver);
        }

        if self.state.column_full(column) {
            return Err(MoveError::ColumnFull(column));
        }

        self.state.place_on_column(column, self.current_colour);

        let colour = self.current_colour;
        self.current_colour = self.current_colour.invert();

        if let Some(winner) = self.check_win(column, colour) {
            self.winner = Some(winner);
        }

        Ok(())
    }

    fn check_win(&self, column: Column, colour: Colour) -> Option<Winner> {
        let height = self.state.column_height(column);

        assert!(height >= 1);
        let last_placed = height - 1;

        const DIRECTIONS: &[(i8, i8, usize)] = &[
            (-1, 1, 0),
            (0, 1, 1),
            (1, 1, 2),
            (-1, 0, 3),
            (1, 0, 3),
            (-1, -1, 2),
            (0, -1, 1),
            (1, -1, 0),
        ];

        // 0:\ 1:| 2:/ 3:-
        let mut count_in_direction = [1; 4];

        let mut stopped_checking_direction = [false; 8];

        for depth in 1..=4 {
            for (i, &(dx, dy, dir_idx)) in DIRECTIONS.into_iter().enumerate() {
                if stopped_checking_direction[i] {
                    continue;
                }

                let check_col = match column.offset(dx * depth) {
                    Some(c) => c,
                    None => continue,
                };
                let check_row = match row_offset(last_placed, dy * depth) {
                    Some(c) => c,
                    None => continue,
                };

                let colour_at_pos = self.state.piece_at(check_col, check_row);

                if colour_at_pos != Some(colour) {
                    stopped_checking_direction[i] = true;
                } else {
                    count_in_direction[dir_idx] += 1;
                }
            }
        }

        for &x in &count_in_direction {
            if x >= 4 {
                return Some(Winner::from_colour(colour));
            }
        }

        // check if the board is full
        for &col in Column::all() {
            if !self.state.column_full(col) {
                return None;
            }
        }

        Some(Winner::Tie)
    }

    fn is_finished(&self) -> bool {
        self.winner.is_some()
    }

    fn winner(&self) -> Option<Winner> {
        self.winner
    }

    fn current_colour(&self) -> Colour {
        self.current_colour
    }

    fn state(&self) -> &Board {
        &self.state
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Fitness {
    Loss = 0,
    Tie,
    Win,
}

impl rubot::Game for Game {
    type Player = Colour;
    type Action = Column;
    type Fitness = Fitness;
    type Actions = Vec<Column>;

    fn actions(&self, player: Self::Player) -> (bool, Self::Actions) {
        let actions = if self.is_finished() {
            vec![]
        } else {
            self.state.allowed_columns()
        };

        (player == self.current_colour(), actions)
    }

    fn execute(&mut self, action: &Self::Action, player: Self::Player) -> Self::Fitness {
        self.make_move(*action).unwrap();

        match self.winner().and_then(Winner::to_colour) {
            None => Fitness::Tie,
            Some(c) if c == player => Fitness::Win,
            _ => Fitness::Loss,
        }
    }

    fn is_upper_bound(&self, fitness: Self::Fitness, _player: Self::Player) -> bool {
        fitness == Fitness::Win
    }

    fn is_lower_bound(&self, fitness: Self::Fitness, _player: Self::Player) -> bool {
        fitness == Fitness::Loss
    }
}

fn perform() {
    let colours = &[Colour::Red, Colour::Yellow];

    let human_player = dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt("Play as")
        .items(&["Red", "Yellow"])
        .interact()
        .unwrap();
    let human_player = colours[human_player];

    let first_player = dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt("Who goes first")
        .items(&["Red", "Yellow"])
        .interact()
        .unwrap();

    let mut game = Game::new(colours[first_player]);

    let mut bot = rubot::Bot::new(Colour::Yellow);

    while !game.is_finished() {
        println!("Game State:");
        game.state().render();

        if game.current_colour() == human_player {
            let items = game.state().allowed_columns();
            let chosen = dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
                .with_prompt("Your turn")
                .items(&items)
                .interact()
                .unwrap();

            game.make_move(items[chosen]).unwrap();
        } else {
            // println!("Yellow's Turn");
            // let items = game.state().allowed_columns();
            // let chosen =
            //     dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
            //         .items(&items)
            //         .interact()
            //         .unwrap();

            // game.make_move(items[chosen]).unwrap();

            println!("Bot's Turn");
            let action = bot.select(&game, Duration::from_secs(5)).unwrap();
            game.make_move(action).unwrap();
        }
    }

    println!("{:?}", game.winner());
}

fn main() {
    perform();
}
