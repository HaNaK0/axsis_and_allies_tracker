use clap::{Parser, Subcommand, ValueEnum};
use log::{error, debug};
use ron::{de::from_reader, ser::{PrettyConfig, to_writer_pretty}};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fmt::Display,
    fs::OpenOptions,
};

const FILE: &str = "state.ron";

type IPC = i32;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct AppArgs {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Setup a new game
    Setup {
        /// The IPC you start out with
        initial_icp: IPC,
    },
    /// Show the current status of the game
    Status,
    /// Add a troop type to current purchase
    Purchase {
        /// The troop type to add to purchase
        troop: Troops,
        /// The ammount to add
        #[arg(default_value_t = 1)]
        ammount: i32,
    },
    /// Remove something from the purchase this round
    Remove {
        /// The troop type to remove from purchase
        troop: Troops,
        /// The ammount to remove
        ammount: Option<i32>,
    },
    /// Checks and Commits the purchase and updates to the new ipc
    Commit {
        /// The ipc you get this round
        ipc: IPC
    }
}

#[derive(ValueEnum, Copy, Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
enum Troops {
    // Army
    Infantery,
    Tank,
    Artillery,
    AAA,
    IC,
    // Airforce
    Fighter,
    Bomber,
    // Navy
    Battleship,
    AircraftCarrier,
    Destroyer,
    Cruiser,
    Submarine,
    Transport,
}

impl Troops {
    const fn get_cost(&self) -> IPC {
        match self {
            Troops::Infantery => 3,
            Troops::Tank => 6,
            Troops::Artillery => 4,
            Troops::AAA => 5,
            Troops::IC => 15,
            Troops::Fighter => 10,
            Troops::Bomber => 12,
            Troops::Battleship => 20,
            Troops::AircraftCarrier => 14,
            Troops::Cruiser => 12,
            Troops::Destroyer => 8,
            Troops::Submarine => 6,
            Troops::Transport => 7,
        }
    }
}

impl Display for Troops {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Troops::Infantery => write!(f, "Infantery"),
            Troops::Tank => write!(f, "Tank"),
            Troops::Artillery => write!(f, "Artillery"),
            Troops::AAA => write!(f, "AAA"),
            Troops::IC => write!(f, "IC"),
            Troops::Fighter => write!(f, "Fighter"),
            Troops::Bomber => write!(f, "Bomber"),
            Troops::Battleship => write!(f, "Battleship"),
            Troops::AircraftCarrier => write!(f, "Aircraft Carrier"),
            Troops::Destroyer => write!(f, "Destroyer"),
            Troops::Cruiser => write!(f, "Cruiser"),
            Troops::Submarine => write!(f, "Submarine"),
            Troops::Transport => write!(f, "Transport"),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct GameState {
    ipc: IPC,
    purchases: HashMap<Troops, i32>,
}

impl GameState {
    fn new(ipc: IPC) -> Self {
        Self {
            ipc,
            purchases: HashMap::default(),
        }
    }

    fn get_total_cost(&self) -> IPC {
        self.purchases.iter()
            .fold(0, |acc, (troop, ammount)| acc + troop.get_cost() * ammount)
    }
}

impl Display for GameState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Current game state:")?;
        writeln!(f, "Purchases:")?;

        let mut cost = 0;

        for (troop, ammount) in self.purchases.iter() {
            cost += troop.get_cost() * ammount;
            writeln!(f, "\t{} : {} á {} ipc", troop, ammount, troop.get_cost())?;
        }

        writeln!(f, "At a total cost of {cost} ipc")?;
        writeln!(f, "Remaining IPC: {}", self.ipc - cost)?;

        Ok(())
    }
}

fn main() {
    env_logger::Builder::default().build();
    debug!("axsis_and_allies_trecker");

    let cli = AppArgs::parse();

    let state = match cli.command {
        Commands::Setup { initial_icp } => Some(GameState::new(initial_icp)),
        Commands::Status => {
            show_status();
            None
        },
        Commands::Purchase { troop, ammount } => add_purchase(troop, ammount),
        Commands::Remove { troop, ammount } => remove_purchase(troop, ammount),
        Commands::Commit { ipc } => commit_purchase(ipc),
    };

    if let Some(state) = state {
        save(state);
    }
}

fn show_status() {
    if let Some(state) = load() {
        println!("{state}")
    }
}

fn add_purchase(troop: Troops, ammount: i32) -> Option<GameState> {
    load().map(|mut state|{
        state.purchases.insert(troop, state.purchases.get(&troop).unwrap_or(&0) + ammount);
        println!("Added a purchase of {} {}s for {}", ammount, troop, troop.get_cost() * ammount);
        println!("Remaining IPC: {}", state.ipc - state.get_total_cost());
        state
    })
}

fn remove_purchase(troop: Troops, ammount: Option<i32>) -> Option<GameState> {
    load().map(|mut state| {
        match ammount {
            Some(ammount) => {
                state.purchases.insert(troop, state.purchases.get(&troop).unwrap_or(&0) - ammount);
                println!("Removing {ammount} {troop}s from purchase")
            }
            None => {
                state.purchases.insert(troop, 0);
                println!("Removing all {troop}s from purchase")
            }
        };

        if state.purchases[&troop] <= 0 {
            state.purchases.remove(&troop);
        }

        state
    })
}

fn commit_purchase(new_ipc: IPC) -> Option<GameState> {
    match load() {
        Some(mut state) => {
            let remaining_ipc = state.ipc - state.get_total_cost();
            if remaining_ipc >= 0 {
                println!("commiting purchases...");
                state.purchases.clear();

                println!("IPC remaining {remaining_ipc}");

                state.ipc = remaining_ipc + new_ipc;
                println!("New IPC total {}", state.ipc);

                Some(state)
            } else {
                println!("You don't have enough IPC to pay for your purchases");
                None
            }
        },
        None => None,
    }
}

fn load() -> Option<GameState> {
    let file = OpenOptions::new().read(true).open(FILE);

    match file {
        Ok(file) => from_reader(file).map_or_else(
            |e| {
                error!("Failed to load game state from file due to error {e:?}");
                None
            },
            |state| Some(state),
        ),
        Err(e) => {
            error!("Failed to load game state from file due to error {e:?}");
            None
        }
    }
}

fn save(state: GameState) {
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(FILE);

    match file {
        Ok(file) => {
            if let Err(e) = to_writer_pretty(file, &state, PrettyConfig::default()) {
                error!("Failed to save state due to err: {e:?}")
            };
        }
        Err(e) => error!("Failed to save state due to err: {e:?}"),
    };
}