//! `CPython`'s numbers, ported once (ADR-090, SPEC-302).
//!
//! The W4 ports read the predecessor's numbers through `CPython`'s own arithmetic, and each differs
//! from the obvious Rust at an edge. Every function here is proved bit for bit against a golden of
//! `CPython` (or of the predecessor's own function), never re-derived.

/// Python's built-in `sum` of floats: Neumaier's compensated sum, the compensation added once at
/// the end and only when it is non-zero and finite, exactly as `builtin_sum` does.
///
/// A plain running sum differs from it on cancellation (`1e16 + 1 - 1e16` is 1, not 0).
#[must_use]
pub fn sum(values: impl IntoIterator<Item = f64>) -> f64 {
    let mut total = 0.0_f64;
    let mut compensation = 0.0_f64;
    for x in values {
        let next = total + x;
        if total.abs() >= x.abs() {
            compensation += (total - next) + x;
        } else {
            compensation += (x - next) + total;
        }
        total = next;
    }
    if compensation != 0.0 && compensation.is_finite() {
        total += compensation;
    }
    total
}

/// `statistics.median` of a list, or `None` for an empty one (where `statistics` raises).
///
/// The middle value of an odd list, or the mean of the two middle values of an even one, taken as
/// `(a + b) / 2` in floats. A list that holds a NaN has no median, and reads NaN (SPEC-302
/// section 5).
#[must_use]
pub fn median(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    if values.iter().any(|x| x.is_nan()) {
        return Some(f64::NAN);
    }
    let mut ordered = values.to_vec();
    ordered.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = ordered.len();
    let middle = n / 2;
    if n % 2 == 1 {
        ordered.get(middle).copied()
    } else {
        Some((ordered.get(middle - 1)? + ordered.get(middle)?) / 2.0)
    }
}

/// The 2^-1074 unit a finite float is a whole multiple of, as `(mantissa, shift)`.
fn units(x: f64) -> (u64, u32) {
    let bits = x.to_bits();
    let exponent = u32::try_from((bits >> 52) & 0x7ff).unwrap_or(0);
    let fraction = bits & ((1_u64 << 52) - 1);
    if exponent == 0 {
        (fraction, 0)
    } else {
        (fraction + (1_u64 << 52), exponent - 1)
    }
}

/// A non-negative integer of base-2^32 limbs, least significant first: wide enough to hold the
/// exact sum of finite floats in units of 2^-1074.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Wide(Vec<u32>);

impl Wide {
    /// Enough limbs for 2^2100 times 2^64 floats.
    const LIMBS: usize = 72;

    fn zero() -> Self {
        Self(vec![0; Self::LIMBS])
    }

    /// Adds `mantissa * 2^shift`.
    fn add(&mut self, mantissa: u64, shift: u32) {
        let at = usize::try_from(shift / 32).unwrap_or(0);
        let mut carry = u128::from(mantissa) << (shift % 32);
        for limb in self.0.iter_mut().skip(at) {
            if carry == 0 {
                break;
            }
            carry += u128::from(*limb);
            *limb = u32::try_from(carry & 0xffff_ffff).unwrap_or(0);
            carry >>= 32;
        }
    }

    /// Subtracts `other`, which must not exceed `self`.
    fn sub(&mut self, other: &Self) {
        let mut borrow = 0_i64;
        for (limb, taken) in self.0.iter_mut().zip(&other.0) {
            let diff = i64::from(*limb) - i64::from(*taken) - borrow;
            borrow = i64::from(diff < 0);
            *limb = u32::try_from(diff.rem_euclid(1 << 32)).unwrap_or(0);
        }
    }

    /// Divides by `divisor`, returning the remainder.
    fn divide(&mut self, divisor: u64) -> u64 {
        let mut remainder = 0_u128;
        for limb in self.0.iter_mut().rev() {
            let current = (remainder << 32) + u128::from(*limb);
            *limb = u32::try_from(current / u128::from(divisor)).unwrap_or(0);
            remainder = current % u128::from(divisor);
        }
        u64::try_from(remainder).unwrap_or(0)
    }

    fn bit(&self, index: usize) -> bool {
        self.0
            .get(index / 32)
            .is_some_and(|limb| (limb >> (index % 32)) & 1 == 1)
    }

    fn bit_length(&self) -> usize {
        (0..Self::LIMBS * 32)
            .rev()
            .find(|index| self.bit(*index))
            .map_or(0, |index| index + 1)
    }

    /// The bits from `from` upward, as a `u64` of at most 54 bits.
    fn bits_from(&self, from: usize, count: usize) -> u64 {
        (0..count).fold(0_u64, |acc, offset| {
            acc + (u64::from(self.bit(from + offset)) << offset)
        })
    }

    /// Whether any bit below `below` is set.
    fn any_below(&self, below: usize) -> bool {
        (0..below).any(|index| self.bit(index))
    }
}

/// 2^`exponent` as a float, for an exponent a float can hold.
fn power_of_two(exponent: i32) -> f64 {
    if exponent >= -1022 {
        f64::from_bits(u64::try_from(exponent + 1023).unwrap_or(0) << 52)
    } else {
        f64::from_bits(1_u64 << u32::try_from(exponent + 1074).unwrap_or(0))
    }
}

/// `statistics.mean`: the exact sum divided by the count, rounded to a float once.
///
/// A list that holds a non-finite value returns the float sum of its non-finite values alone, as
/// `statistics` does. `None` for an empty list, where `statistics` raises.
#[must_use]
pub fn mean(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let count = u64::try_from(values.len()).unwrap_or(u64::MAX);
    if values.iter().any(|x| !x.is_finite()) {
        // `statistics` sums only the non-finite values, and a non-finite sum over a positive count
        // is itself, so it returns that sum undivided.
        return Some(values.iter().filter(|x| !x.is_finite()).sum::<f64>());
    }
    let mut positive = Wide::zero();
    let mut negative = Wide::zero();
    for x in values {
        let (mantissa, shift) = units(*x);
        if x.is_sign_negative() {
            negative.add(mantissa, shift);
        } else {
            positive.add(mantissa, shift);
        }
    }
    let negate = negative_is_larger(&positive, &negative);
    let mut total = if negate {
        negative.clone()
    } else {
        positive.clone()
    };
    total.sub(if negate { &positive } else { &negative });
    let remainder = total.divide(count);
    let magnitude = round_to_float(&total, remainder, count);
    Some(if negate { -magnitude } else { magnitude })
}

/// Whether the negative parts outweigh the positive ones.
fn negative_is_larger(positive: &Wide, negative: &Wide) -> bool {
    negative.0.iter().rev().gt(positive.0.iter().rev())
}

/// The float nearest `(quotient + remainder / divisor) * 2^-1074`, ties to even.
fn round_to_float(quotient: &Wide, remainder: u64, divisor: u64) -> f64 {
    let length = quotient.bit_length();
    let unit = f64::from_bits(1);
    if length <= 53 {
        let mut whole = quotient.bits_from(0, 53);
        let twice = u128::from(remainder) * 2;
        if twice > u128::from(divisor) || (twice == u128::from(divisor) && whole % 2 == 1) {
            whole += 1;
        }
        #[allow(clippy::cast_precision_loss, reason = "at most 2^53, exactly a float")]
        return whole as f64 * unit;
    }
    let shift = length - 53;
    let mut mantissa = quotient.bits_from(shift, 53);
    let half = quotient.bit(shift - 1);
    let below_half = quotient.any_below(shift - 1) || remainder > 0;
    if half && (below_half || mantissa % 2 == 1) {
        mantissa += 1;
    }
    let exponent = i32::try_from(shift).unwrap_or(0) - 1074;
    #[allow(clippy::cast_precision_loss, reason = "at most 2^53, exactly a float")]
    let value = mantissa as f64;
    value * power_of_two(exponent)
}

/// Python's `round(x, ndigits)` of a float: the nearest multiple of 10^-`ndigits` to the EXACT
/// binary value of `x`, a tie going to the even last digit.
///
/// `round(2.675, 2)` is 2.67 because the float is 2.67499...; the decimal expansion of the float
/// is taken exactly, rounded as text, and read back to the nearest float.
/// A rounded value beyond the largest float is an infinity, where CPython raises (SPEC-302
/// section 5).
#[must_use]
pub fn round(x: f64, ndigits: i32) -> f64 {
    if !x.is_finite() || x == 0.0 || ndigits > 323 {
        return x;
    }
    if ndigits < -308 {
        return 0.0_f64.copysign(x);
    }
    let expansion = format!("{:.1074}", x.abs());
    let (whole, fraction) = expansion
        .split_once('.')
        .unwrap_or((expansion.as_str(), ""));
    let mut digits: Vec<u8> = whole
        .bytes()
        .chain(fraction.bytes())
        .map(|b| b - b'0')
        .collect();
    let kept = i64::try_from(whole.len()).unwrap_or(0) + i64::from(ndigits);
    let pad = usize::try_from(-kept).unwrap_or(0);
    digits.splice(0..0, std::iter::repeat_n(0, pad));
    let kept = usize::try_from(kept).unwrap_or(0);
    let rest = digits.split_off(kept.min(digits.len()));
    let first = rest.first().copied().unwrap_or(0);
    let beyond = rest.iter().skip(1).any(|d| *d != 0);
    let last_odd = digits.last().is_some_and(|d| d % 2 == 1);
    if first > 5 || (first == 5 && (beyond || last_odd)) {
        let mut at = digits.len();
        loop {
            if at == 0 {
                digits.insert(0, 1);
                break;
            }
            at -= 1;
            if let Some(digit) = digits.get_mut(at) {
                if *digit == 9 {
                    *digit = 0;
                    continue;
                }
                *digit += 1;
            }
            break;
        }
    }
    let text: String = digits.iter().map(|d| char::from(b'0' + d)).collect();
    let text = if text.is_empty() {
        "0".to_owned()
    } else {
        text
    };
    let magnitude: f64 = format!("{text}e{}", -i64::from(ndigits))
        .parse()
        .unwrap_or(0.0);
    magnitude.copysign(x)
}

/// The predecessor's nearest-rank percentile: the value at rank `max(1, ceil(pct * n))` of the
/// ascending list, and 0 for an empty one.
///
/// `pct * n` is a float product (`0.9 * 70` is 63.00000000000001, rank 64), so the rank is the
/// ceiling of that float and no integer shortcut. A `pct` outside `[0, 1]` reads an end of the
/// list, where the predecessor's index would raise.
#[must_use]
pub fn percentile(values: &[i64], pct: f64) -> i64 {
    let mut ordered = values.to_vec();
    ordered.sort_unstable();
    let n = ordered.len();
    if n == 0 {
        return 0;
    }
    #[allow(
        clippy::cast_precision_loss,
        reason = "the length of a list is far below 2^53"
    )]
    let product = pct * n as f64;
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss,
        reason = "clamped to 1..=n before the cast, and a list length is far below 2^53"
    )]
    let rank = (product.ceil().clamp(1.0, n as f64)) as usize;
    ordered.get(rank - 1).copied().unwrap_or(0)
}

const STATE_LEN: usize = 624;
const SHIFT_LEN: usize = 397;

/// `CPython`'s `random.Random`: a Mersenne Twister seeded as `random.Random(seed)` seeds it.
#[derive(Debug, Clone)]
pub struct PyRandom {
    state: [u32; STATE_LEN],
    index: usize,
}

impl PyRandom {
    /// A generator seeded with a non-negative integer: its 32-bit words, least significant first,
    /// are the key of `init_by_array`; zero is the one-word key `[0]`.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        let low = u32::try_from(seed & 0xffff_ffff).unwrap_or(0);
        let high = u32::try_from(seed >> 32).unwrap_or(0);
        let key: Vec<u32> = if high == 0 {
            vec![low]
        } else {
            vec![low, high]
        };
        let mut state = [0_u32; STATE_LEN];
        state[0] = 19_650_218;
        for i in 1..STATE_LEN {
            let previous = state[i - 1];
            state[i] = 1_812_433_253_u32
                .wrapping_mul(previous ^ (previous >> 30))
                .wrapping_add(u32::try_from(i).unwrap_or(0));
        }
        let mut i = 1;
        let mut j = 0;
        for _ in 0..STATE_LEN.max(key.len()) {
            let previous = state[i - 1];
            state[i] = (state[i] ^ (previous ^ (previous >> 30)).wrapping_mul(1_664_525))
                .wrapping_add(key[j])
                .wrapping_add(u32::try_from(j).unwrap_or(0));
            i += 1;
            j += 1;
            if i >= STATE_LEN {
                state[0] = state[STATE_LEN - 1];
                i = 1;
            }
            if j >= key.len() {
                j = 0;
            }
        }
        for _ in 0..STATE_LEN - 1 {
            let previous = state[i - 1];
            state[i] = (state[i] ^ (previous ^ (previous >> 30)).wrapping_mul(1_566_083_941))
                .wrapping_sub(u32::try_from(i).unwrap_or(0));
            i += 1;
            if i >= STATE_LEN {
                state[0] = state[STATE_LEN - 1];
                i = 1;
            }
        }
        state[0] = 0x8000_0000;
        Self {
            state,
            index: STATE_LEN,
        }
    }

    fn twist(&mut self) {
        for k in 0..STATE_LEN {
            let upper = self.state[k] & 0x8000_0000;
            let lower = self.state[(k + 1) % STATE_LEN] & 0x7fff_ffff;
            let y = upper + lower;
            let mut next = self.state[(k + SHIFT_LEN) % STATE_LEN] ^ (y >> 1);
            if y & 1 == 1 {
                next ^= 0x9908_b0df;
            }
            self.state[k] = next;
        }
        self.index = 0;
    }

    fn word(&mut self) -> u32 {
        if self.index >= STATE_LEN {
            self.twist();
        }
        let mut y = self.state[self.index];
        self.index += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c_5680;
        y ^= (y << 15) & 0xefc6_0000;
        y ^ (y >> 18)
    }

    /// The next 53-bit float in `[0, 1)`: `(a * 2^26 + b) / 2^53` from two words.
    pub fn random(&mut self) -> f64 {
        let a = f64::from(self.word() >> 5);
        let b = f64::from(self.word() >> 6);
        (a * 67_108_864.0 + b) * (1.0 / 9_007_199_254_740_992.0)
    }

    /// `random.choices(population, k=k)`: `k` picks, each `population[floor(random() * n)]`.
    /// `None` for an empty population with `k` above zero, where `choices` raises.
    pub fn choices<'a, T>(&mut self, population: &'a [T], k: usize) -> Option<Vec<&'a T>> {
        #[allow(
            clippy::cast_precision_loss,
            reason = "the length of a list is far below 2^53"
        )]
        let n = population.len() as f64;
        let mut picks = Vec::with_capacity(k);
        for _ in 0..k {
            #[allow(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                reason = "random() is in [0, 1), so the floor is within the list"
            )]
            let index = (self.random() * n).floor() as usize;
            picks.push(population.get(index)?);
        }
        Some(picks)
    }
}

/// The Lanczos numerator coefficients of `math.lgamma`, as `CPython` carries them (N = 13).
#[allow(
    clippy::excessive_precision,
    reason = "CPython's coefficients, digit for digit"
)]
const LANCZOS_NUMERATOR: [f64; 13] = [
    23_531_376_880.410_759_688_572_007_674_451_636_754_734_846_804_940,
    42_919_803_642.649_098_768_957_899_047_001_988_850_926_355_848_959,
    35_711_959_237.355_668_049_440_185_451_547_166_705_960_488_635_843,
    17_921_034_426.037_209_699_919_755_754_458_931_112_671_403_265_390,
    6_039_542_586.352_028_005_064_291_644_307_297_921_069_938_842_070_8,
    1_439_720_407.311_721_673_663_223_072_794_912_393_971_548_578_677_2,
    248_874_557.862_054_156_511_460_386_413_229_423_216_321_251_278_01,
    31_426_415.585_400_194_380_614_231_628_318_205_362_874_684_987_640,
    2_876_370.628_935_372_441_225_409_051_620_849_613_599_114_537_876_8,
    186_056.265_395_223_495_040_294_989_716_045_699_282_207_842_363_28,
    8_071.672_002_365_816_210_638_002_902_272_250_613_821_851_632_502_4,
    210.824_277_751_579_345_872_509_733_920_713_362_711_669_695_802_91,
    2.506_628_274_631_000_270_164_908_177_133_837_338_626_431_079_340_8,
];

/// The Lanczos denominator coefficients of `math.lgamma`.
const LANCZOS_DENOMINATOR: [f64; 13] = [
    0.0,
    39_916_800.0,
    120_543_840.0,
    150_917_976.0,
    105_258_076.0,
    45_995_730.0,
    13_339_535.0,
    2_637_558.0,
    357_423.0,
    32_670.0,
    1_925.0,
    66.0,
    1.0,
];

/// `CPython`'s `lanczos_g`.
#[allow(
    clippy::excessive_precision,
    reason = "CPython's constant, digit for digit"
)]
const LANCZOS_G: f64 = 6.024_680_040_776_729_583_740_234_375;

fn lanczos_sum(x: f64) -> f64 {
    let mut numerator = 0.0_f64;
    let mut denominator = 0.0_f64;
    if x < 5.0 {
        for (n, d) in LANCZOS_NUMERATOR.iter().zip(&LANCZOS_DENOMINATOR).rev() {
            numerator = numerator * x + n;
            denominator = denominator * x + d;
        }
    } else {
        for (n, d) in LANCZOS_NUMERATOR.iter().zip(&LANCZOS_DENOMINATOR) {
            numerator = numerator / x + n;
            denominator = denominator / x + d;
        }
    }
    numerator / denominator
}

/// `math.lgamma` of a finite argument above zero, as `CPython`'s Lanczos port computes it.
///
/// `None` at or below zero and for a non-finite argument, where the port is not asked for a value
/// (negative arguments are left to the slice that needs them), and where the result overflows (from
/// about 2.6e305), where CPython raises.
#[must_use]
pub fn lgamma(x: f64) -> Option<f64> {
    if !x.is_finite() || x <= 0.0 {
        return None;
    }
    #[allow(clippy::float_cmp, reason = "an exact integer test, as CPython's is")]
    let integral = x == x.floor();
    if integral && x <= 2.0 {
        return Some(0.0);
    }
    if x < 1e-20 {
        return Some(-x.ln());
    }
    let mut r = lanczos_sum(x).ln() - LANCZOS_G;
    r += (x - 0.5) * ((x + LANCZOS_G - 0.5).ln() - 1.0);
    if r.is_infinite() {
        return None;
    }
    Some(r)
}
