use crate::core::card::{Card, Owner, Rank, Suit};
use crate::core::combinations::combinations;

fn show_combination(combo: &[Card]) {
    for card in combo {
        print!("{}/ ", card);
    }
    print!("\n");
}

pub fn combinations_demo() {
    let hand = vec![
        Card {
            rank: Rank::Ace,
            suit: Suit::Spades,
            owner: Owner::Player,
        },
        Card {
            rank: Rank::King,
            suit: Suit::Hearts,
            owner: Owner::Player,
        },
        Card {
            rank: Rank::Queen,
            suit: Suit::Diamonds,
            owner: Owner::Community,
        },
        Card {
            rank: Rank::Jack,
            suit: Suit::Clubs,
            owner: Owner::Community,
        },
        Card {
            rank: Rank::Ten,
            suit: Suit::Spades,
            owner: Owner::Community,
        },
        Card {
            rank: Rank::Nine,
            suit: Suit::Hearts,
            owner: Owner::Community,
        },
        Card {
            rank: Rank::Eight,
            suit: Suit::Diamonds,
            owner: Owner::Community,
        },
    ];

    println!("HAND --------------------------------");
    show_combination(&hand);
    println!("-------------------------------------");

    for k in [2, 3, 5] {
        let combos = combinations(&hand, k);
        println!("\nCombinations of size {} ({}):", k, combos.len());
        for combo in &combos {
            show_combination(combo);
        }
    }

    let empty = combinations(&hand, 8);
    println!("\nCombinations of size 8 ({}):", empty.len());
}
