//! Tiered cost-vs-quality harness for the DSpark dual-engine pipeline.
//!
//! Offline mode (default) simulates the three deployment configurations over a
//! deterministic task stream so the ECONOMIC ACCOUNTING of each tier can be
//! inspected without any API keys:
//!
//!   1. flagship-only : flagship writes every task from scratch
//!   2. cheap-only    : cheap model writes everything, no verification
//!   3. tiered        : cheap drafts N candidates -> cheap PPT ranking ->
//!      cheap contract verification -> flagship refines ONLY
//!      escalated residual cases
//!
//!   cargo run -p dspark-core --example cost_quality_harness
//!
//! All probabilities and prices are ASSUMPTIONS declared below; edit them to
//! match your provider pricing and measured model quality. This tool accounts
//! costs; it does not fabricate accuracy results.

use dspark::engine::tournament_comparison_count;

const TASKS: usize = 200;
const N_TRAJECTORIES: usize = 3;
const K_PIVOTS_REQUESTED: usize = 2;

// ---- Assumptions (edit me) -------------------------------------------------
/// $ per cheap-tier API call (draft / rank / verify), e.g. deepseek-chat scale.
const CHEAP_CALL_USD: f64 = 0.0015;
/// $ per flagship-tier API call (refinement), e.g. frontier reasoning model.
const FLAGSHIP_CALL_USD: f64 = 0.0300;
/// Probability a single cheap draft satisfies the I/O contracts.
const P_CHEAP_DRAFT_OK: f64 = 0.55;
/// Probability the flagship produces a correct refinement given counterexamples.
const P_FLAGSHIP_FIX: f64 = 0.88;
/// Probability the contract verifier correctly flags an incorrect winner.
const P_VERIFY_CATCH: f64 = 0.80;
// -----------------------------------------------------------------------------

/// Tiny deterministic PRNG (xorshift64*) so runs are reproducible without deps.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
    fn bernoulli(&mut self, p: f64) -> bool {
        self.next() < p
    }
}

#[derive(Default)]
struct Tally {
    pass: usize,
    cheap_calls: usize,
    flagship_calls: usize,
}

impl Tally {
    fn cost(&self) -> f64 {
        self.cheap_calls as f64 * CHEAP_CALL_USD + self.flagship_calls as f64 * FLAGSHIP_CALL_USD
    }
}

fn main() {
    println!(
        "DSpark tiered cost harness: {} simulated tasks (N={}, requested k={})",
        TASKS, N_TRAJECTORIES, K_PIVOTS_REQUESTED
    );
    println!(
        "Assumptions: cheap=${:.4}/call, flagship=${:.4}/call, p(draft ok)={:.2}, p(fix)={:.2}, p(catch)={:.2}\n",
        CHEAP_CALL_USD, FLAGSHIP_CALL_USD, P_CHEAP_DRAFT_OK, P_FLAGSHIP_FIX, P_VERIFY_CATCH
    );

    let mut lcg = Lcg(0xD5_2026_0822u64.wrapping_mul(2654435761));
    let mut flagship_only = Tally::default();
    let mut cheap_only = Tally::default();
    let mut tiered = Tally::default();
    let mut escalations = 0usize;
    // Exact PPT ranking traffic the tiered config pays per run.
    let ranking_calls = tournament_comparison_count(N_TRAJECTORIES, K_PIVOTS_REQUESTED);

    for _task in 0..TASKS {
        // 1. Flagship-only: one expensive generation attempt per task.
        flagship_only.flagship_calls += 1;
        if lcg.bernoulli(P_FLAGSHIP_FIX.max(0.90)) {
            flagship_only.pass += 1;
        }

        // 2. Cheap-only: one cheap attempt, no verification safety net.
        cheap_only.cheap_calls += 1;
        if lcg.bernoulli(P_CHEAP_DRAFT_OK) {
            cheap_only.pass += 1;
        }

        // 3. Tiered: N cheap drafts -> cheap ranking -> cheap verification ->
        //    flagship ONLY when residual risk survives.
        tiered.cheap_calls += N_TRAJECTORIES; // drafting
        tiered.cheap_calls += ranking_calls; // PPT ring + pivot comparisons
        let correct_drafts = (0..N_TRAJECTORIES)
            .filter(|_| lcg.bernoulli(P_CHEAP_DRAFT_OK))
            .count();
        tiered.cheap_calls += 1; // contract verification of the elected winner

        if correct_drafts > 0 {
            // A valid candidate exists; winner is correct unless the tournament
            // elects a wrong one AND verification misses it.
            let wrong_election = lcg.next();
            let caught = lcg.bernoulli(P_VERIFY_CATCH);
            if wrong_election < 0.10 && !caught {
                // Silent failure path: mis-ranked and uncaught.
            } else if wrong_election < 0.10 && caught {
                tiered.flagship_calls += 1; // escalate flagged winner
                escalations += 1;
                if lcg.bernoulli(P_FLAGSHIP_FIX) {
                    tiered.pass += 1;
                }
            } else {
                tiered.pass += 1;
            }
        } else {
            // No draft passed: escalation policy sends the case to the flagship.
            tiered.flagship_calls += 1;
            escalations += 1;
            if lcg.bernoulli(P_FLAGSHIP_FIX) {
                tiered.pass += 1;
            }
        }
    }

    println!("{:<16} {:>6} {:>12} {:>12} {:>14}", "config", "pass%", "cheap calls", "flagship", "$ total");
    for (name, t) in [
        ("flagship-only", &flagship_only),
        ("cheap-only", &cheap_only),
        ("tiered", &tiered),
    ] {
        println!(
            "{:<16} {:>6.1} {:>12} {:>12} {:>14.4}",
            name,
            t.pass as f64 / TASKS as f64 * 100.0,
            t.cheap_calls,
            t.flagship_calls,
            t.cost()
        );
    }
    println!(
        "\nTiered escalations to flagship: {}/{} tasks ({:.1}%)",
        escalations,
        TASKS,
        escalations as f64 / TASKS as f64 * 100.0
    );
    println!("Ranking calls per run (PPT): {} vs all-pairs {}", ranking_calls, {
        let n = N_TRAJECTORIES;
        n * (n - 1) / 2
    });
}
