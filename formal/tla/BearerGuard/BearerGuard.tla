------------------------------ MODULE BearerGuard ------------------------------
\* @phx covers crates/mcp/src/limiter.rs anchor=decide digest=sha256:bcefd88248e57233b19a103ff347af2c194f6f1aa0c5789739fcf1fcfae2cc0f
\* @phx covers crates/mcp/src/guard.rs anchor=admit digest=sha256:f5d836baf1f741ad7eec12d9cca354a3e217f8787675893f1f8593ad5f322629
\* @phx covers crates/mcp/src/guard.rs anchor=authorize digest=sha256:e49a37c49fee067b96c789218a107d9b34e2980ec7d2f469ac10997beac36e33
\* @phx cites #158
\* @phx property GrantedAlwaysAdmitted ramp=report
\* @phx property FreshFailuresAtMostMax ramp=report
\* @phx property BucketsAtMostMax ramp=report
\* @phx property OneRefusalForEveryCause ramp=report
\* @phx witness witness/a-limiter-read-before-the-match.cfg kills=GrantedAlwaysAdmitted
\* @phx witness witness/a-count-and-record-under-two-locks.cfg kills=FreshFailuresAtMostMax
\* @phx witness witness/a-recorder-that-never-evicts.cfg kills=BucketsAtMostMax
\* @phx witness witness/a-limited-refusal-with-its-own-word.cfg kills=OneRefusalForEveryCause
(***************************************************************************)
\* The MCP guard's shared limiter (#158): NReq requests in flight at once (SPEC-119 R4 allows up
\* to eight), each presenting a token and asking for a scope, over ONE limiter whose buckets every
\* request reads and writes, while the clock moves.
\*
\* A request is decided in one step of its own:
\* - guard.rs::admit answers a token that matches a grant before the limiter reads anything
\*   (scope core), and guard.rs::authorize answers a matched token's scope the same way ("a
\*   presented token whose grant holds the scope is allowed before the limiter reads anything",
\*   R13); a granted token is admitted, so a request's admission and its scope check are one
\*   decision here;
\* - otherwise limiter.rs::decide runs with the token's bucket: under ONE lock it reads the
\*   bucket's fresh failures, answers rate_limited recording nothing when there are MaxFailures or
\*   more (keeping the fresh ones and the bucket's place), and else records the failure, keeps the
\*   newest HistoryCap, moves the bucket newest and evicts the oldest beyond MaxBuckets;
\* - every refusal is one response: guard.rs builds it in one function from constants (R11), and
\*   a scope refusal carries the same word (R12).
\*
\* What is abstracted:
\* - a token is one of four: the granted token, two wrong tokens and no token at all, each with a
\*   bucket of its own (the bucket is a digest's prefix; no two of these collide);
\* - the grant holds core alone, and law is a scope it lacks; a second grant adds nothing the
\*   limiter can see, since a matched in-scope token never reaches it;
\* - the clock is the kernel's wall clock, which may move either way between two decisions: the
\*   Clock action sets any instant of 0..MaxTime; a failure is fresh while now - t < Window, a
\*   negative age included (limiter.rs::decide);
\* - the request's header parse and the digest match are a pure function of the token, so they
\*   are the token's kind here;
\* - each request runs once; the window, the count, the caps and the clock are small.
\*
\* Four defect switches, each FALSE in one witness: LimiterAfterMatch (the match is read before
\* the limiter), OneLock (the count and the record are one critical section), Evict (the recorder
\* evicts the oldest bucket beyond the cap) and OneWord (a limited refusal answers the same word).
(***************************************************************************)
EXTENDS Integers, Sequences, FiniteSets

CONSTANTS LimiterAfterMatch, OneLock, Evict, OneWord, NReq, MaxFailures, MaxBuckets, Window, MaxTime

Requests == 1..NReq
Instants == 0..MaxTime
Tokens == {"granted", "wrong1", "wrong2", "absent"}
Buckets == Tokens
Scopes == {"core", "law"}
GrantScopes == {"core"}
HistoryCap == 2 * MaxFailures
Places == {"idle", "decide", "act", "done"}
Outcomes == {"none", "allowed", "denied", "rate_limited"}
Answers == {"none", "admitted", "unauthorized", "rate limited"}

VARIABLES
    pc,       \* each request's place
    tok,      \* each request's token
    scope,    \* each request's scope
    seen,     \* each request's count of fresh failures, read under the first of two locks
    outcome,  \* each request's outcome
    answer,   \* each request's response
    hist,     \* each bucket's failure instants, oldest first
    order,    \* the buckets, oldest first
    now       \* the clock

vars == <<pc, tok, scope, seen, outcome, answer, hist, order, now>>

Range(s) == {s[i] : i \in DOMAIN s}

TypeOK ==
    /\ pc \in [Requests -> Places]
    /\ tok \in [Requests -> Tokens]
    /\ scope \in [Requests -> Scopes]
    /\ seen \in [Requests -> 0..HistoryCap]
    /\ outcome \in [Requests -> Outcomes]
    /\ answer \in [Requests -> Answers]
    /\ hist \in [Buckets -> Seq(Instants)]
    /\ \A b \in Buckets : Len(hist[b]) <= HistoryCap
    /\ order \in Seq(Buckets)
    /\ now \in Instants

Init ==
    /\ pc = [r \in Requests |-> "idle"]
    /\ tok = [r \in Requests |-> "absent"]
    /\ scope = [r \in Requests |-> "core"]
    /\ seen = [r \in Requests |-> 0]
    /\ outcome = [r \in Requests |-> "none"]
    /\ answer = [r \in Requests |-> "none"]
    /\ hist = [b \in Buckets |-> <<>>]
    /\ order = <<>>
    /\ now = 0

\* The bucket's failures still fresh at the current instant.
Fresh(b) == SelectSeq(hist[b], LAMBDA t : now - t < Window)

\* A matched token whose grant holds the scope.
Matched(r) == tok[r] = "granted" /\ scope[r] \in GrantScopes

\* The response a refusal of outcome o answers.
Refusal(o) == IF o = "rate_limited" /\ ~OneWord THEN "rate limited" ELSE "unauthorized"

\* Request r ends with outcome o.
Finish(r, o) ==
    /\ pc' = [pc EXCEPT ![r] = "done"]
    /\ outcome' = [outcome EXCEPT ![r] = o]
    /\ answer' = [answer EXCEPT ![r] = IF o = "allowed" THEN "admitted" ELSE Refusal(o)]

\* limiter.rs::decide's limited arm: nothing recorded; the stale failures are dropped and the
\* bucket keeps its place.
Limit(r) ==
    /\ Finish(r, "rate_limited")
    /\ hist' = [hist EXCEPT ![tok[r]] = Fresh(tok[r])]
    /\ UNCHANGED order

\* limiter.rs::decide's recording arm: the fresh failures and this one, the newest HistoryCap
\* kept; the bucket moved newest; the oldest beyond MaxBuckets evicted (or with the defect, kept).
Record(r) ==
    LET b == tok[r]
        kept == Append(Fresh(b), now)
        capped == IF Len(kept) > HistoryCap
                  THEN SubSeq(kept, Len(kept) - HistoryCap + 1, Len(kept))
                  ELSE kept
        moved == Append(SelectSeq(order, LAMBDA x : x /= b), b)
        left == IF Evict /\ Len(moved) > MaxBuckets
                THEN SubSeq(moved, Len(moved) - MaxBuckets + 1, Len(moved))
                ELSE moved
    IN /\ Finish(r, "denied")
       /\ order' = left
       /\ hist' = [x \in Buckets |->
                      IF x = b THEN capped ELSE IF x \in Range(left) THEN hist[x] ELSE <<>>]

\* A request arrives with a token and a scope.
Arrive(r) ==
    /\ pc[r] = "idle"
    /\ \E t \in Tokens, s \in Scopes :
          /\ tok' = [tok EXCEPT ![r] = t]
          /\ scope' = [scope EXCEPT ![r] = s]
    /\ pc' = [pc EXCEPT ![r] = "decide"]
    /\ UNCHANGED <<seen, outcome, answer, hist, order, now>>

\* guard.rs::admit and guard.rs::authorize: the match first, then limiter.rs::decide with the
\* token's bucket (or with the defect, the limiter read before the match).
Decide(r) ==
    /\ pc[r] = "decide"
    /\ LET fresh == Len(Fresh(tok[r]))
       IN IF ~LimiterAfterMatch /\ fresh >= MaxFailures
          THEN /\ Limit(r)
               /\ UNCHANGED seen
          ELSE IF Matched(r)
          THEN /\ Finish(r, "allowed")
               /\ UNCHANGED <<seen, hist, order>>
          ELSE IF OneLock
          THEN /\ IF fresh >= MaxFailures THEN Limit(r) ELSE Record(r)
               /\ UNCHANGED seen
          ELSE /\ seen' = [seen EXCEPT ![r] = fresh]
               /\ pc' = [pc EXCEPT ![r] = "act"]
               /\ UNCHANGED <<outcome, answer, hist, order>>
    /\ UNCHANGED <<tok, scope, now>>

\* The defect's second lock: the act on a count another request may have changed since.
Act(r) ==
    /\ pc[r] = "act"
    /\ IF seen[r] >= MaxFailures THEN Limit(r) ELSE Record(r)
    /\ UNCHANGED <<tok, scope, seen, now>>

\* The wall clock moves to any instant while a request is still in flight.
Clock ==
    /\ \E r \in Requests : pc[r] /= "done"
    /\ \E t \in Instants \ {now} : now' = t
    /\ UNCHANGED <<pc, tok, scope, seen, outcome, answer, hist, order>>

\* Every request has been answered.
Finished ==
    /\ \A r \in Requests : pc[r] = "done"
    /\ UNCHANGED vars

Next ==
    \/ \E r \in Requests : Arrive(r) \/ Decide(r) \/ Act(r)
    \/ Clock
    \/ Finished

Spec == Init /\ [][Next]_vars

\* A granted token is answered for its own scope, whatever its own refusals before ("a presented
\* token whose grant holds the scope is allowed before the limiter reads anything", R13;
\* guard.rs::admit, guard.rs::authorize).
GrantedAlwaysAdmitted ==
    \A r \in Requests : (pc[r] = "done" /\ Matched(r)) => outcome[r] = "allowed"

\* No bucket holds more than MaxFailures fresh failures ("with 5 or more fresh failures the
\* outcome is rate_limited and nothing is recorded", R13; limiter.rs::decide).
FreshFailuresAtMostMax == \A b \in Buckets : Len(Fresh(b)) <= MaxFailures

\* The limiter keeps at most MaxBuckets buckets ("the oldest bucket beyond 512 is evicted", R13;
\* limiter.rs::decide).
BucketsAtMostMax == Len(order) <= MaxBuckets

\* Every refusal answers the one word, whatever its cause ("an absent, empty, malformed, wrong or
\* rate-limited bearer is answered byte for byte the same", R11; R12).
OneRefusalForEveryCause ==
    \A r \in Requests : (pc[r] = "done" /\ outcome[r] /= "allowed") => answer[r] = "unauthorized"

\* Reachability, beside the witnesses: a request is limited; a bucket is evicted; a granted token
\* is allowed while its own bucket holds MaxFailures fresh failures.
ARequestIsLimited == \E r \in Requests : outcome[r] = "rate_limited"
ABucketIsEvicted ==
    /\ Len(order) = MaxBuckets
    /\ \E r \in Requests : outcome[r] = "denied" /\ tok[r] \notin Range(order)
AGrantedTokenIsAllowedOverItsFullBucket ==
    /\ Len(Fresh("granted")) >= MaxFailures
    /\ \E r \in Requests : outcome[r] = "allowed" /\ tok[r] = "granted"

=============================================================================
