use std::fmt::{Display, Formatter};

pub enum GameType {
    TexasHoldemPoker = 0,
    OmahaPoker
}


impl Display for GameType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result{
        match self {
            Self::TexasHoldemPoker => write!(f, "Texas Holdem Poker"),
            Self::OmahaPoker => write!(f, "Omaha Poker")
        }
    }
}
