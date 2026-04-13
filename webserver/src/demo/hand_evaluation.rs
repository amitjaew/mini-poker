use crate::core::card::{ Suit, Rank, Card, Owner };
use crate::core::hand::{ evaluate_hand, evaluate_hand_omaha, compare_hands, HandCompare };
use crate::core::game::GameType;

fn show_hand(hand: &[Card]) {
    for card in hand {
        print!("{}/ ", card);
    }
    print!("\n");
}

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

fn print_compare_result(result: Result<HandCompare, &'static str>, player_names: &[&str]) {
    match result {
        Ok(HandCompare::Winner(idx)) => println!("Winner: {}", player_names[idx]),
        Ok(HandCompare::Tie(indexes)) => {
            print!("Tie between: ");
            for idx in &indexes { print!("{} ", player_names[*idx]); }
            println!();
        }
        Err(e) => println!("Error: {}", e),
    }
}

pub fn compare_hands_holdem_demo() {
    // 5 community cards shared, each player has 2 hole cards (7 cards total per hand)
    // Player 1: A-K hole cards => Royal Flush with Q-J-T community spades
    // Player 2: 2-7 offsuit => High card
    // Player 3: 9-9 pocket => Pair of nines

    let community = [
        Card { rank: Rank::Queen, suit: Suit::Spades,   owner: Owner::Community },
        Card { rank: Rank::Jack,  suit: Suit::Spades,   owner: Owner::Community },
        Card { rank: Rank::Ten,   suit: Suit::Spades,   owner: Owner::Community },
        Card { rank: Rank::Five,  suit: Suit::Diamonds, owner: Owner::Community },
        Card { rank: Rank::Six,   suit: Suit::Hearts,   owner: Owner::Community },
    ];

    let player1 = vec![
        Card { rank: Rank::Ace,  suit: Suit::Spades, owner: Owner::Player },
        Card { rank: Rank::King, suit: Suit::Spades, owner: Owner::Player },
        community[0], community[1], community[2], community[3], community[4],
    ];
    let player2 = vec![
        Card { rank: Rank::Two,   suit: Suit::Clubs,    owner: Owner::Player },
        Card { rank: Rank::Seven, suit: Suit::Diamonds, owner: Owner::Player },
        community[0], community[1], community[2], community[3], community[4],
    ];
    let player3 = vec![
        Card { rank: Rank::Nine, suit: Suit::Hearts,   owner: Owner::Player },
        Card { rank: Rank::Nine, suit: Suit::Diamonds, owner: Owner::Player },
        community[0], community[1], community[2], community[3], community[4],
    ];

    let player_names = ["Player 1 (A-K spades)", "Player 2 (2-7 offsuit)", "Player 3 (9-9 pocket)"];

    println!("\nTEXAS HOLD'EM COMPARE ---------------");
    for (i, hand) in [&player1, &player2, &player3].iter().enumerate() {
        print!("{}: ", player_names[i]);
        show_hand(hand);
    }
    println!("-------------------------------------");

    let result = compare_hands(vec![player1, player2, player3], GameType::TexasHoldemPoker);
    print_compare_result(result, &player_names);
}

pub fn compare_hands_omaha_demo() {
    // 5 community cards + 4 hole cards (Owner::Player) per hand
    // Player 1: A-K-Q-J spades => Royal Flush (A-K hole + Q-J-T community)
    // Player 2: 2-3-4-5 mixed => Straight (A-2-3-4-5 low)
    // Player 3: K-K-Q-Q mixed => Full House (K-K-K via community K + pair Q)

    let community = [
        Card { rank: Rank::Ten,   suit: Suit::Spades,   owner: Owner::Community },
        Card { rank: Rank::King,  suit: Suit::Hearts,   owner: Owner::Community },
        Card { rank: Rank::Queen, suit: Suit::Clubs,    owner: Owner::Community },
        Card { rank: Rank::Five,  suit: Suit::Diamonds, owner: Owner::Community },
        Card { rank: Rank::Ace,   suit: Suit::Clubs,    owner: Owner::Community },
    ];

    // Player 1: A-K spades hole => Royal Flush (A K Q J T all spades — but Q/T spades not in community)
    // Simplified: just pick interesting hands
    // Player 1: K-Q-J-T spades hole + community (Ten Spades, King Hearts, Queen Clubs, Five Diamonds, Ace Clubs)
    // Best: straight A-K-Q-J-T using hole K-Q and community A,T + ... let's keep it simple

    let p1: Vec<Card> = vec![
        Card { rank: Rank::Ace,  suit: Suit::Spades, owner: Owner::Player },
        Card { rank: Rank::King, suit: Suit::Spades, owner: Owner::Player },
        Card { rank: Rank::Jack, suit: Suit::Spades, owner: Owner::Player },
        Card { rank: Rank::Nine, suit: Suit::Clubs,  owner: Owner::Player },
        community[0], community[1], community[2], community[3], community[4],
    ];
    let p2: Vec<Card> = vec![
        Card { rank: Rank::Two,   suit: Suit::Hearts,   owner: Owner::Player },
        Card { rank: Rank::Three, suit: Suit::Diamonds, owner: Owner::Player },
        Card { rank: Rank::Four,  suit: Suit::Clubs,    owner: Owner::Player },
        Card { rank: Rank::Seven, suit: Suit::Spades,   owner: Owner::Player },
        community[0], community[1], community[2], community[3], community[4],
    ];
    let p3: Vec<Card> = vec![
        Card { rank: Rank::King,  suit: Suit::Diamonds, owner: Owner::Player },
        Card { rank: Rank::King,  suit: Suit::Clubs,    owner: Owner::Player },
        Card { rank: Rank::Queen, suit: Suit::Hearts,   owner: Owner::Player },
        Card { rank: Rank::Eight, suit: Suit::Spades,   owner: Owner::Player },
        community[0], community[1], community[2], community[3], community[4],
    ];

    let player_names = [
        "Player 1 (A-K-J-9 spades/clubs)",
        "Player 2 (2-3-4-7 mixed)",
        "Player 3 (K-K-Q-8 mixed)",
    ];

    println!("\nOMAHA COMPARE -----------------------");
    for (i, hand) in [&p1, &p2, &p3].iter().enumerate() {
        print!("{}: ", player_names[i]);
        show_hand(hand);
    }
    println!("-------------------------------------");

    let result = compare_hands(vec![p1, p2, p3], GameType::OmahaPoker);
    print_compare_result(result, &player_names);
}
