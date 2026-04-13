# Pending Fixes

## 1. Long-term — Replace evaluator with a lookup table

**Problem:**
The current evaluator uses branching logic and multiple passes. For equity calculators that evaluate millions of hands this becomes a bottleneck.

**Fix:**
Implement a lookup-table evaluator (Cactus Kev or Two Plus Two style):
- Encode each card as a 32-bit integer (rank + suit bits)
- Use a pre-computed hash table mapping any 5-card combination to a hand rank integer
- Evaluation becomes a handful of array lookups with no branching

This is a full rewrite of `evaluate_hand`, not a patch.
