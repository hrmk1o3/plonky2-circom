use plonky2::field::extension::quadratic::QuadraticExtension;
use plonky2::field::extension::Extendable;
use plonky2::field::goldilocks_field::GoldilocksField;

use plonky2::field::types::{Field, Field64, PrimeField64};
use plonky2::hash::hash_types::HashOut;
use plonky2::hash::hashing::{compress, hash_n_to_hash_no_pad, PlonkyPermutation};
use plonky2::hash::poseidon::{PoseidonHash, PoseidonPermutation, SPONGE_RATE, SPONGE_WIDTH, Permuter};
use plonky2::iop::target::{BoolTarget, Target};
use plonky2::plonk::circuit_builder::CircuitBuilder;
use plonky2::plonk::config::{AlgebraicHasher, GenericConfig, Hasher};
use poseidon_permutation::bindings::permute;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct PoseidonBN128Permutation<F> {
    pub state: [F; SPONGE_WIDTH],
}

impl<T: Copy + Default> AsRef<[T]> for PoseidonBN128Permutation<T> {
    fn as_ref(&self) -> &[T] {
        &self.state
    }
}

type F = GoldilocksField;

impl PlonkyPermutation<F> for PoseidonBN128Permutation<F> {
    const RATE: usize = SPONGE_RATE;
    const WIDTH: usize = SPONGE_WIDTH;

    fn new<I: IntoIterator<Item = F>>(elts: I) -> Self {
        let mut perm = Self {
            state: [F::default(); SPONGE_WIDTH],
        };
        perm.set_from_iter(elts, 0);
        perm
    }

    fn set_elt(&mut self, elt: F, idx: usize) {
        self.state[idx] = elt;
    }

    fn set_from_slice(&mut self, elts: &[F], start_idx: usize) {
        let begin = start_idx;
        let end = start_idx + elts.len();
        self.state[begin..end].copy_from_slice(elts);
    }

    fn set_from_iter<I: IntoIterator<Item = F>>(&mut self, elts: I, start_idx: usize) {
        for (s, e) in self.state[start_idx..].iter_mut().zip(elts) {
            *s = e;
        }
    }

    fn squeeze(&self) -> &[F] {
        &self.state[..Self::RATE]
    }

    fn permute(&mut self) {
        // self.state = F::permute(self.state);
        let input = self.state;
        let state = input.map(|v| v.to_canonical_u64());
        let state = unsafe {
            let h = permute(
                state[0], state[1], state[2], state[3], state[4], state[5], state[6], state[7],
                state[8], state[9], state[10], state[11],
            );

            [
                h.r0, h.r1, h.r2, h.r3, h.r4, h.r5, h.r6, h.r7, h.r8, h.r9, h.r10, h.r11,
            ]
        };

        for v in state {
            assert!(v < F::ORDER);
        }

        self.state =
            state.map(|r| F::from_canonical_u64(if r >= F::ORDER { r - F::ORDER } else { r }));
    }
}

impl PlonkyPermutation<Target> for PoseidonBN128Permutation<Target> {
    const RATE: usize = SPONGE_RATE;
    const WIDTH: usize = SPONGE_WIDTH;

    fn new<I: IntoIterator<Item = Target>>(elts: I) -> Self {
        let mut perm = Self {
            state: [Target::default(); SPONGE_WIDTH],
        };
        perm.set_from_iter(elts, 0);
        perm
    }

    fn set_elt(&mut self, elt: Target, idx: usize) {
        self.state[idx] = elt;
    }

    fn set_from_slice(&mut self, elts: &[Target], start_idx: usize) {
        let begin = start_idx;
        let end = start_idx + elts.len();
        self.state[begin..end].copy_from_slice(elts);
    }

    fn set_from_iter<I: IntoIterator<Item = Target>>(&mut self, elts: I, start_idx: usize) {
        for (s, e) in self.state[start_idx..].iter_mut().zip(elts) {
            *s = e;
        }
    }

    fn permute(&mut self) {
        todo!()
    }

    fn squeeze(&self) -> &[Target] {
        &self.state[..Self::RATE]
    }
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct PoseidonBN128Hash;
impl Hasher<F> for PoseidonBN128Hash {
    const HASH_SIZE: usize = 4 * 8;
    type Hash = HashOut<F>;
    type Permutation = PoseidonBN128Permutation<F>;

    fn hash_no_pad(input: &[F]) -> Self::Hash {
        hash_n_to_hash_no_pad::<F, Self::Permutation>(input)
    }

    fn two_to_one(left: Self::Hash, right: Self::Hash) -> Self::Hash {
        compress::<F, Self::Permutation>(left, right)
    }
}

// TODO: this is a work around. Still use Goldilocks based Poseidon for algebraic PoseidonBN128Hash.
impl AlgebraicHasher<F> for PoseidonBN128Hash {
    type AlgebraicPermutation = PoseidonBN128Permutation<Target>;

    fn permute_swapped<const D: usize>(
        inputs: Self::AlgebraicPermutation,
        swap: BoolTarget,
        builder: &mut CircuitBuilder<F, D>,
    ) -> Self::AlgebraicPermutation
    where
        F: Extendable<D>,
    {
        let output = PoseidonHash::permute_swapped(
            PoseidonPermutation::new(inputs.as_ref().iter().cloned()),
            swap,
            builder,
        );

        PoseidonBN128Permutation {
            state: output.as_ref().try_into().unwrap(),
        }
    }
}

/// Configuration using Poseidon over the Goldilocks field.
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct PoseidonBN128GoldilocksConfig;

impl GenericConfig<2> for PoseidonBN128GoldilocksConfig {
    type F = GoldilocksField;
    type FE = QuadraticExtension<Self::F>;
    type Hasher = PoseidonBN128Hash;
    type InnerHasher = PoseidonBN128Hash;
}

#[cfg(test)]
mod tests {
    use anyhow::Result;
    use plonky2::field::types::Field;
    use plonky2::plonk::config::{GenericConfig, Hasher, PoseidonGoldilocksConfig};

    use crate::config::PoseidonBN128Hash;

    #[test]
    fn test_poseidon_bn128() -> Result<()> {
        const D: usize = 2;
        type C = PoseidonGoldilocksConfig;
        type F = <C as GenericConfig<D>>::F;

        let v = [
            8917524657281059100u64,
            13029010200779371910,
            16138660518493481604,
            17277322750214136960,
            1441151880423231822,
        ]
        .map(F::from_canonical_u64);
        let h = PoseidonBN128Hash::hash_no_pad(&v);
        assert_eq!(h.elements[0].0, 16736853722845225729u64);
        assert_eq!(h.elements[1].0, 1446699130810517790u64);
        assert_eq!(h.elements[2].0, 15445626857806971868u64);
        assert_eq!(h.elements[3].0, 6331160477881736675u64);

        Ok(())
    }
}
