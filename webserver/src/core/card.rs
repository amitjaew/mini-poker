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
pub const CARD_RANKS: [Rank; 13] = [
    Rank::Two,
    Rank::Three,
    Rank::Four,
    Rank::Five,
    Rank::Six,
    Rank::Seven,
    Rank::Eight,
    Rank::Nine,
    Rank::Ten,
    Rank::Jack,
    Rank::Queen,
    Rank::King,
    Rank::Ace
];

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

pub const fn build_deck() -> [Card; 52] {
    let mut deck = [Card { rank: Rank::Two, suit: Suit::Clubs, owner: Owner::Community }; 52];
    let mut i = 0;
    let mut suit_index = 0;
    while suit_index < 4 {
        let suit = CARD_SUITS[suit_index];
        let mut rank_index = 0;
        while rank_index < 13 {
            let rank = CARD_RANKS[rank_index];
            deck[i] = Card { rank, suit, owner: Owner::Community };
            i += 1;
            rank_index += 1;
        }
        suit_index += 1;
    }
    deck
}

pub const DECK: [Card; 52] = build_deck();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Suit {
    Clubs = 0,
    Diamonds,
    Hearts,
    Spades
}
pub const CARD_SUITS: [Suit; 4] = [
    Suit::Clubs,
    Suit::Diamonds,
    Suit::Hearts,
    Suit::Spades
];

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
