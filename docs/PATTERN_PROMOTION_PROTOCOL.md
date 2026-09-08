# Pattern Promotion Protocol

Local repetition is not architecture. Promote a pattern only when it is useful
across boundaries and its owner is unambiguous.

1. Record the proposed pattern in `docs/PATTERN_CANDIDATES.md` with problem,
   owner, exact example paths, and evidence.
2. Prove the smallest working use at the owning boundary. Do not create a
   generic abstraction for a hypothetical caller.
3. Check that the pattern preserves one mechanism owner, typed facades, reactor
   ownership, and renderer/native boundaries.
4. Review the proposal with the affected owner before adding it to canonical
   guidance.
5. Promote by updating the owner index or standard, then retain the candidate's
   evidence and decision.

Rejected or unproven patterns remain candidates. Wildcard exceptions and hidden
compatibility layers cannot be promoted.
