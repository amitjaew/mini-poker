use crate::core::card::{ Card, Rank };
use crate::core::game::GameType;
use std::cmp::{min, max};
use std::fmt::{ Display, Formatter };
use std::result::Result;

pub fn show_hand(hand: &[Card]) {
    for card in hand {
        print!("{} {} / ", card.rank, card.suit);
    }
    print!("\n");
}

pub enum HandType {
    HighCard = 0,
    Pair,
    TwoPair,
    ThreeOfAKind,
    Straight,
    Flush,
    FullHouse,
    FourOfAKind,
    StraightFlush,
    RoyalFlush
}

impl Display for HandType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HighCard => write!(f, "High Card"),
            Self::Pair => write!(f, "Pair"),
            Self::TwoPair => write!(f, "Two Pair"),
            Self::ThreeOfAKind => write!(f, "Three of a Kind"),
            Self::Straight => write!(f, "Straight"),
            Self::Flush => write!(f, "Flush"),
            Self::FullHouse => write!(f, "Full House"),
            Self::FourOfAKind => write!(f, "Four of a Kind"),
            Self::StraightFlush => write!(f, "Straight Flush"),
            Self::RoyalFlush => write!(f, "Royal Flush")
        }
    }
}

fn get_straight(ranks: &Vec<u8>) -> Option<Vec<u8>> {
    let n: usize = ranks.len();
    let mut continuity_count = 0;

    for i in 0..n-1 {
        let delta = ranks[i] - ranks[i+1];
        if delta == 0 { continue; }
        else if delta == 1 { continuity_count += 1; }
        else { continuity_count = 0; }

        if continuity_count == 4 {
            let mut scale = Vec::with_capacity(5);
            let base = ranks[i+1] + 4;
            for j in 0..5 { scale.push(base - j); }

            for j in 0..5 { print!("{} ", scale[j]); }
            return Some(scale);
        }
    }
    if  ranks[0] == 12 &&
        ranks[n-1] == 0 &&
        continuity_count == 3
    {
        let mut scale = Vec::with_capacity(5);
        scale.push(12);
        for j in 0..4 { scale.push(3 - j); }
        return Some(scale);
    }
    
    return None;
}

pub fn evaluate_hand(hand: &mut [Card], gametype: GameType) -> Result<(HandType, Vec<u8>), &'static str> {
    hand.sort();
    hand.reverse();

    match gametype {
        GameType::TexasHoldemPoker => {
            if hand.len() != 7 { return Err("Texas Holdem requires 7 cards"); }
        },
        GameType::OmahaPoker => {
            if hand.len() != 10 { return Err("Omaha poker requires 10 cards"); }
        }
    }
    /*
     * TODO Manage Omaha Poker requirements:
     *  Hand should be composed by 2 hole cards and 3 community cards
     *  THIS SHIT REQUIRES REFACTORING :(
     */

    let mut sorted_card_rank: Vec<u8> = Vec::with_capacity(5);
    for _ in 0..5 { sorted_card_rank.push(0); }

    let mut is_flush = false;
    let mut is_straight = false;
    let mut is_four_of_kind = false;
    let mut is_full_house = false;
    let mut is_three_of_kind = false;
    let mut is_two_pair = false;
    let mut is_pair = false;
    let mut suit_count = Vec::with_capacity(4);
    
    let mut pair_rank: u8 = 0;
    let mut three_rank: u8 = 0;

    for _ in 0..4 { suit_count.push(0); }

    // Flush check
    for i in 0..hand.len() {
        let card = hand[i];
        if suit_count[card.suit as usize] == 4 {
            is_flush = true;

            let mut filtered_hand: Vec<u8>  = hand.iter()
                                    .filter( |_card| _card.suit == card.suit )
                                    .map( |_card| _card.rank as u8 )
                                    .collect();
            // Flush + scale check (royal/straight)
            match get_straight(&mut filtered_hand) {
                Some(straight) => {
                    for i in 0..5 { sorted_card_rank[i] = straight[i]; }
                    is_straight = true;
                },
                None => {
                    for i in 0..5 { sorted_card_rank[i] =  filtered_hand[i]; }
                }
            }
            break;
        }
        suit_count[card.suit as usize] += 1;
    }

    if is_flush && is_straight && sorted_card_rank[0] == Rank::Ace as u8 {
        return Ok((HandType::RoyalFlush, sorted_card_rank));
    }
    else if is_flush && is_straight {
        return Ok((HandType::StraightFlush, sorted_card_rank));
    }

    let mut card_count: Vec<u8> = Vec::with_capacity(13);
    for _ in 0..13 { card_count.push(0); }
    for i in 0..hand.len() { card_count[hand[i].rank as usize] += 1; }
    
    for i in (0..card_count.len()).rev() {
        let count: u8 = card_count[i];

        if count == 0 { continue; }
        else if count == 4 {
            is_four_of_kind = true;
            let filtered_hand = hand.iter()
                                    .filter(|card| card.rank as usize != i)
                                    .map(|card| card.rank as u8);
            for j in 0..4 { sorted_card_rank[j] = i as u8; }

            match filtered_hand.max() {
                Some(val) => sorted_card_rank[4] = val,
                None => sorted_card_rank[4] = 0
            }
            break; // Best case on remaining scenarios, doesnt require greedy search
        }
        else if (count == 3 && is_pair) || (count == 2 && is_three_of_kind) || (count == 3 && is_three_of_kind){
            is_full_house = true;
            if count == 2 /* && is_three_of_kind */ { pair_rank = i as u8; }
            else if count == 3 && is_pair { three_rank = i as u8; }
            else {
                pair_rank = min(three_rank, i as u8);
                three_rank = max(three_rank, i as u8);
            }

            for j in 0..3 { sorted_card_rank[j] = three_rank; }
            for j in 3..5 { sorted_card_rank[j] = pair_rank; }
        }
        else if count == 2 && is_pair && !is_flush {
            is_two_pair = true;
            if i > pair_rank.into() {
                for j in 0..2 { sorted_card_rank[j] = i as u8; }
                for j in 2..4 { sorted_card_rank[j] = pair_rank; }
            }
            else {
                for j in 0..2 { sorted_card_rank[j] = pair_rank; }
                for j in 2..4 { sorted_card_rank[j] = i as u8; }
            }

            let filtered_hand = hand.iter()
                                    .filter(|card| card.rank as usize != i && card.rank as u8 != pair_rank)
                                    .map(|card| card.rank as u8);
            match filtered_hand.max() {
                Some(val) => sorted_card_rank[4] = val,
                None => sorted_card_rank[4] = 0
            }
        }
        else if count == 3 {
            is_three_of_kind = true;
            three_rank = i as u8;
        }
        else if count == 2 {
            is_pair = true;
            pair_rank = i as u8;
        }

    }

    if is_four_of_kind {
        return Ok((HandType::FourOfAKind, sorted_card_rank));
    }
    else if is_full_house {
        return Ok((HandType::FullHouse, sorted_card_rank));
    }
    else if is_flush {
        return Ok((HandType::Flush, sorted_card_rank));
    }

    // Straight check
    let ranks = hand.iter()
                    .map(|card| card.rank as u8)
                    .collect();
    match get_straight(&ranks) {
        Some(straight) => {
           for i in 0..5 { sorted_card_rank[i] = straight[i]; }
           is_straight = true;
        },
        None => {}
    }
    if is_straight {
        return Ok((HandType::Straight, sorted_card_rank));
    }
    else if is_three_of_kind {
        let temp : Vec<u8> = hand.iter()
                                    .filter(|card| card.rank as u8 != three_rank)
                                    .map(|card| card.rank as u8)
                                    .collect();
        for i in 0..3 { sorted_card_rank[i] = three_rank as u8; }
        for i in 0..2 { sorted_card_rank[3+i] = temp[i] }
        return Ok((HandType::ThreeOfAKind, sorted_card_rank));
    }
    else if is_two_pair {
        return Ok((HandType::TwoPair, sorted_card_rank));
    }
    else if is_pair {
        let temp : Vec<u8> = hand.iter()
                                    .filter(|card| card.rank as u8 != pair_rank)
                                    .map(|card| card.rank as u8)
                                    .collect();
        for i in 0..2 { sorted_card_rank[i] = pair_rank; }
        for i in 0..3 { sorted_card_rank[2+i] = temp[temp.len()-i-1] }
        return Ok((HandType::Pair, sorted_card_rank));
    }
    else {
        let mut temp : Vec<u8> = hand.iter()
                                    .map(|card| card.rank as u8)
                                    .collect();
        temp.sort();
        for i in 0..5 { sorted_card_rank[i] = temp[temp.len()-i-1]; }
        return Ok((HandType::HighCard, sorted_card_rank));
    }
}
