use std::fmt::{Display, Formatter};
use std::cmp::Ordering ;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rank {
    Two = 0,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
    Ace
}

impl Display for Rank {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Two => write!(f, "Two"),
            Self::Three => write!(f, "Tree"),
            Self::Four => write!(f, "Four"),
            Self::Five => write!(f, "Five"),
            Self::Six =>  write!(f, "Six"),
            Self::Seven => write!(f, "Seven"),
            Self::Eight => write!(f, "Eight"),
            Self::Nine => write!(f, "Nine"),
            Self::Ten => write!(f, "Ten"),
            Self::Jack => write!(f, "Jack"),
            Self::Queen => write!(f, "Quen"),
            Self::King => write!(f, "King"),
            Self::Ace => write!(f, "Ace"),
        }
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Suit {
    Clubs = 0,
    Diamonds,
    Hearts,
    Spades
}

impl Display for Suit {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Clubs => write!(f, "Clubs"),
            Self::Diamonds => write!(f, "Diamonds"),
            Self::Hearts => write!(f, "Hearts"),
            Self::Spades => write!(f, "Spades")
        }
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Owner {
    Player = 0,
    Community
}


impl Display for Owner {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Player => write!(f, "Player"),
            Self::Community => write!(f, "Community")
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Card {
    pub rank: Rank,
    pub suit: Suit,
    pub owner: Owner
}

impl Ord for Card {
    fn cmp(&self, other:&Self) -> Ordering {
        self.rank.cmp(&other.rank)
    }
}

impl PartialOrd for Card {
    fn partial_cmp(&self, other:&Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
