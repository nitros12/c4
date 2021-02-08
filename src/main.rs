#![feature(type_alias_impl_trait)]

use std::convert::TryInto;
use std::time::Duration;

use bitvec::prelude::*;
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

    fn to_bool(self) -> bool {
        match self {
            Colour::Red => true,
            Colour::Yellow => false,
        }
    }

    fn from_bool(v: bool) -> Colour {
        if v {
            Colour::Red
        } else {
            Colour::Yellow
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
    G,
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
            Column::G => "G",
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
            Column::G,
        ];

        ALL
    }
}

// const fn max(a: usize, b: usize) -> usize {
//     if a < b {
//         b
//     } else {
//         a
//     }
// }

#[derive(Debug, Clone, PartialEq, Eq)]
struct Board {
    // eventually: heights: [u8; max(BOARD_HEIGHT, BOARD_WIDTH)],
    heights: [u8; BOARD_WIDTH],
    present: bitarr![for BOARD_HEIGHT * BOARD_WIDTH],
    tiles: bitarr![for BOARD_HEIGHT * BOARD_WIDTH],
    gravity_down: bool,
}

struct AllowedColumnsIterator {
    allowed: bitarr![for BOARD_WIDTH],
}

impl AllowedColumnsIterator {
    fn from_board(board: &Board) -> Self {
        let mut allowed = bitarr![0; BOARD_WIDTH];

        for col in Column::all() {
            if !board.column_full(*col) {
                allowed.set(col.to_idx(), true);
            }
        }

        Self { allowed }
    }

    fn new_empty() -> Self {
        Self {
            allowed: Default::default(),
        }
    }
}

impl IntoIterator for AllowedColumnsIterator {
    type Item = Column;

    type IntoIter = impl Iterator<Item = Column>;

    fn into_iter(self) -> Self::IntoIter {
        self.allowed.into_iter().enumerate().filter_map(|(idx, c)| {
            if c {
                Some((idx as u8).try_into().unwrap())
            } else {
                None
            }
        })
    }
}

impl Board {
    fn new() -> Self {
        Self {
            heights: Default::default(),
            present: Default::default(),
            tiles: Default::default(),
            gravity_down: true,
        }
    }

    fn column_height(&self, column: Column) -> u8 {
        self.heights[column.to_idx()]
    }

    fn column_full(&self, column: Column) -> bool {
        self.column_height(column) >= BOARD_HEIGHT as u8
    }

    fn index_of(column: Column, height: u8) -> usize {
        column.to_idx() * BOARD_HEIGHT + height as usize
    }

    fn place_on_column(&mut self, column: Column, colour: Colour) {
        let height = self.column_height(column);
        let height = if self.gravity_down {
            height
        } else {
            BOARD_HEIGHT as u8 - (height + 1)
        };

        let idx = Board::index_of(column, height);
        self.tiles.set(idx, colour.to_bool());
        self.present.set(idx, true);
        self.heights[column.to_idx()] += 1;
    }

    fn piece_at(&self, column: Column, height: u8) -> Option<Colour> {
        let idx = Board::index_of(column, height);
        if self.present[idx] {
            Some(Colour::from_bool(self.tiles[idx]))
        } else {
            None
        }
    }

    fn allowed_columns(&self) -> AllowedColumnsIterator {
        AllowedColumnsIterator::from_board(self)
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

        for c in Column::all() {
            print!("{}", c);
        }

        println!("");
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
    flipping: bool,
    round: u8,
}

impl Game {
    fn new(starting_colour: Colour, flipping: bool) -> Self {
        Self {
            state: Board::new(),
            current_colour: starting_colour,
            winner: None,
            flipping,
            round: 0,
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

        self.current_colour = self.current_colour.invert();

        let height = self.state.column_height(column) - 1;

        if let Some(winner) = self.check_win(column, height) {
            self.winner = Some(winner);
        }

        if self.winner.is_some() {
            return Ok(());
        }

        self.round += 1;

        if self.round == 2 && self.flipping {
            self.round = 0;
            self.flip()
        }

        if let Some(winner) = self.check_win_all() {
            self.winner = Some(winner);
        }

        Ok(())
    }

    fn flip(&mut self) {
        for &column in Column::all() {
            let idx = Board::index_of(column, 0);

            if self.state.column_height(column) == 0 {
                continue;
            }

            let shift = BOARD_HEIGHT - self.state.column_height(column) as usize;

            let present = &mut self.state.present[idx..idx + BOARD_HEIGHT];
            let tiles = &mut self.state.tiles[idx..idx + BOARD_HEIGHT];

            // println!("tiles before {:?} {} {}", present, column, shift);

            if self.state.gravity_down {
                // going up
                present.shift_right(shift);
                tiles.shift_right(shift);
            } else {
                // going down
                present.shift_left(shift);
                tiles.shift_left(shift);
            }

            // println!("tiles after {:?}", present);
        }

        self.state.gravity_down = !self.state.gravity_down;
    }

    fn check_win_all(&self) -> Option<Winner> {
        for &c in Column::all() {
            for h in 0..BOARD_HEIGHT {
                if let Some(win) = self.check_win(c, h as u8) {
                    return Some(win);
                }
            }
        }

        None
    }

    fn check_win(&self, column: Column, height: u8) -> Option<Winner> {
        let colour = match self.state.piece_at(column, height) {
            Some(c) => c,
            None => return None,
        };

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
                let check_row = match row_offset(height, dy * depth) {
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
    type Actions = AllowedColumnsIterator;

    fn actions(&self, player: Self::Player) -> (bool, Self::Actions) {
        let actions = if self.is_finished() {
            AllowedColumnsIterator::new_empty()
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
    let player_opts = &[Some(Colour::Red), Some(Colour::Yellow), None];

    let human_player = dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt("Play as")
        .items(&["Red", "Yellow", "Bot v Bot"])
        .interact()
        .unwrap();
    let human_player = player_opts[human_player];

    let first_player = dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt("Who goes first")
        .items(&["Red", "Yellow"])
        .interact()
        .unwrap();

    let think_time: u64 = dialoguer::Input::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt("Bot think time")
        .default(5)
        .interact()
        .unwrap();

    let flipping = dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
        .with_prompt("Gravity change")
        .items(&["Never", "Every two rounds"])
        .interact()
        .unwrap();
    let flipping = flipping == 1;

    let mut game = Game::new(colours[first_player], flipping);

    let (red_bot, yellow_bot) = match human_player {
        Some(Colour::Red) => (false, true),
        Some(Colour::Yellow) => (true, false),
        None => (true, true),
    };

    let mut red_bot = if red_bot {
        Some(rubot::Bot::new(Colour::Red))
    } else {
        None
    };

    let mut yellow_bot = if yellow_bot {
        Some(rubot::Bot::new(Colour::Yellow))
    } else {
        None
    };

    while !game.is_finished() {
        println!("Game State:");
        game.state().render();

        if Some(game.current_colour()) == human_player {
            let items = game
                .state()
                .allowed_columns()
                .into_iter()
                .collect::<Vec<_>>();
            let chosen = dialoguer::Select::with_theme(&dialoguer::theme::ColorfulTheme::default())
                .with_prompt("Your turn")
                .items(&items)
                .interact()
                .unwrap();

            game.make_move(items[chosen]).unwrap();
        } else {
            println!("Bot's Turn");
            let bot = if game.current_colour() == Colour::Red {
                red_bot.as_mut().unwrap()
            } else {
                yellow_bot.as_mut().unwrap()
            };
            let action = bot.select(&game, Duration::from_secs(think_time)).unwrap();
            game.make_move(action).unwrap();
        }
    }

    game.state().render();

    println!("{:?}", game.winner());
}

fn main() {
    perform();
}
