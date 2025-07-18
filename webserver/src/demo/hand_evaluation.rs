use crate::core::card::{ Suit, Rank, Card, Owner };
use crate::core::hand::{ show_hand, evaluate_hand };
use crate::core::game::GameType;

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
     
     match evaluate_hand(&mut hand, GameType::TexasHoldemPoker){
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
