# Pending Fixes

## 1. `evaluate_hand` — Heap allocations per call

**File:** `webserver/src/core/hand.rs`

**Problem:**
Every call to `evaluate_hand` allocates 5–6 `Vec`s on the heap (`suit_count`, `card_count`, `sorted_card_rank`, `filtered_hand`, `ranks`, `temp`). Since `evaluate_hand` is called 60 times per Omaha hand evaluation, this compounds quickly if running equity calculations.

**Fix:**
Replace fixed-size `Vec`s with stack arrays where the size is known at compile time:
```rust
// instead of:
let mut suit_count = Vec::with_capacity(4);
// use:
let mut suit_count = [0u8; 4];

// instead of:
let mut card_count: Vec<u8> = Vec::with_capacity(13);
// use:
let mut card_count = [0u8; 13];

// instead of:
let mut sorted_card_rank: Vec<u8> = Vec::with_capacity(5);
// use:
let mut sorted_card_rank = [0u8; 5];
```
Return `[u8; 5]` instead of `Vec<u8>` from `evaluate_hand` and `evaluate_hand_omaha`.

---

## 2. `evaluate_hand` — Debug print inside `get_straight`

**File:** `webserver/src/core/hand.rs:60`

**Problem:**
There is a stray debug print that fires every time a straight is found:
```rust
for j in 0..5 { print!("{} ", scale[j]); }
```

**Fix:**
Delete that line entirely.

---

## 3. `evaluate_hand` — `get_straight` called twice

**File:** `webserver/src/core/hand.rs`

**Problem:**
`get_straight` is called once inside the flush branch (~line 111) and again later for the general straight check (~line 208). If the hand is not a flush, the straight scan runs twice on the same data.

**Fix:**
Run `get_straight` once before both checks and store the result:
```rust
let straight = get_straight(&ranks);

if is_flush {
    match straight { ... }  // StraightFlush / RoyalFlush
}
// later...
match straight { ... }  // plain Straight
```

---

## 4. Long-term — Replace evaluator with a lookup table

**Problem:**
The current evaluator uses branching logic and multiple passes. For equity calculators that evaluate millions of hands this becomes a bottleneck.

**Fix:**
Implement a lookup-table evaluator (Cactus Kev or Two Plus Two style):
- Encode each card as a 32-bit integer (rank + suit bits)
- Use a pre-computed hash table mapping any 5-card combination to a hand rank integer
- Evaluation becomes a handful of array lookups with no branching

This is a full rewrite of `evaluate_hand`, not a patch.
