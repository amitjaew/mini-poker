use crate::core::card::{ Suit, Rank, Card, Owner };
use crate::core::hand::{ evaluate_hand, evaluate_hand_omaha, show_hand };

pub fn hand_evaluation_demo() {
     let mut hand = vec![
         Card { rank: Rank::Ace, suit: Suit::Diamonds, owner: Owner::Community},
         Card { rank: Rank::Ace, suit: Suit::Diamonds, owner: Owner::Community},
         Card { rank: Rank::Ace, suit: Suit::Diamonds, owner: Owner::Community},
         Card { rank: Rank::Ten, suit: Suit::Diamonds, owner: Owner::Community},
         Card { rank: Rank::Ten, suit: Suit::Diamonds, owner: Owner::Community},
         Card { rank: Rank::Ten, suit: Suit::Diamonds, owner: Owner::Community},
         Card { rank: Rank::King, suit: Suit::Diamonds, owner: Owner::Community},
     ];

     println!("HAND --------------------------------");
     show_hand(&hand);
     println!("-------------------------------------");

     match evaluate_hand(&mut hand){
         Ok((hand_type, sorted_ranks)) =>  {
             println!("Hand Type: {}", hand_type);
             print!("Sorted rank values: ");
             for i in 0..sorted_ranks.len() { print!("{} ", sorted_ranks[i]); }
             print!("\n");
         },
         Err(message) => {
             println!("Hand evaluation error");
             println!("{}", message);
         }
     }
}

pub fn omaha_evaluation_demo() {
    // 4 hole cards (Player) + 5 community cards
    let mut hand = vec![
        Card { rank: Rank::Ace,   suit: Suit::Spades,   owner: Owner::Player },
        Card { rank: Rank::King,  suit: Suit::Spades,   owner: Owner::Player },
        Card { rank: Rank::Two,   suit: Suit::Hearts,   owner: Owner::Player },
        Card { rank: Rank::Three, suit: Suit::Clubs,    owner: Owner::Player },
        Card { rank: Rank::Queen, suit: Suit::Spades,   owner: Owner::Community },
        Card { rank: Rank::Jack,  suit: Suit::Spades,   owner: Owner::Community },
        Card { rank: Rank::Ten,   suit: Suit::Spades,   owner: Owner::Community },
        Card { rank: Rank::Five,  suit: Suit::Diamonds, owner: Owner::Community },
        Card { rank: Rank::Six,   suit: Suit::Hearts,   owner: Owner::Community },
    ];

    println!("OMAHA HAND --------------------------");
    show_hand(&hand);
    println!("-------------------------------------");

    match evaluate_hand_omaha(&mut hand) {
        Ok((hand_type, sorted_ranks)) => {
            println!("Best Hand Type: {}", hand_type);
            print!("Sorted rank values: ");
            for i in 0..sorted_ranks.len() { print!("{} ", sorted_ranks[i]); }
            print!("\n");
        },
        Err(message) => {
            println!("Omaha evaluation error");
            println!("{}", message);
        }
    }
}
