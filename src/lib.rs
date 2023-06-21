extern crate rand;

use rand::{thread_rng, RngCore};

#[cfg(test)]
mod tests {
    use super::SecretData;
    #[test]
    fn it_works() {}

    #[test]
    fn it_generates_coefficients() {
        let secret_data = SecretData::with_secret("Hello, world!", 3);
        assert_eq!(secret_data.coefficients.len(), 13);
    }

    #[test]
    fn it_rejects_share_id_under_1() {
        let secret_data = SecretData::with_secret("Hello, world!", 3);
        let d = secret_data.get_share(0);
        assert!(d.is_err());
    }

    #[test]
    fn it_issues_shares() {
        let secret_data = SecretData::with_secret("Hello, world!", 3);

        let s1 = secret_data.get_share(1).unwrap();
        println!("Share: {:?}", s1);
        assert!(secret_data.is_valid_share(&s1));
    }

    #[test]
    fn it_repeatedly_issues_shares() {
        let secret_data = SecretData::with_secret("Hello, world!", 3);

        let s1 = secret_data.get_share(1).unwrap();
        println!("Share: {:?}", s1);
        assert!(secret_data.is_valid_share(&s1));

        let s2 = secret_data.get_share(1).unwrap();
        assert_eq!(s1, s2);
    }

    #[test]
    fn it_can_recover_secret() {
        let s1 = vec![1, 184, 190, 251, 87, 232, 39, 47, 17, 4, 36, 190, 245];
        let s2 = vec![2, 231, 107, 52, 138, 34, 221, 9, 221, 67, 79, 33, 16];
        let s3 = vec![3, 23, 176, 163, 177, 165, 218, 113, 163, 53, 7, 251, 196];

        let new_secret = SecretData::recover_secret(3, vec![s1, s2, s3]).unwrap();

        assert_eq!(&new_secret[..], "Hello World!");
    }

    #[test]
    fn it_can_recover_a_generated_secret() {
        let secret_data = SecretData::with_secret("Hello, world!", 3);

        let s1 = secret_data.get_share(1).unwrap();
        println!("s1: {:?}", s1);
        let s2 = secret_data.get_share(2).unwrap();
        println!("s2: {:?}", s2);
        let s3 = secret_data.get_share(3).unwrap();
        println!("s3: {:?}", s3);

        let new_secret = SecretData::recover_secret(3, vec![s1, s2, s3]).unwrap();

        assert_eq!(&new_secret[..], "Hello, world!");
    }

    #[test]
    fn it_requires_enough_shares() {
        fn try_recover(n: u8, shares: &Vec<Vec<u8>>) -> Option<String> {
            let shares = shares.iter().take(n as usize).cloned().collect::<Vec<_>>();
            SecretData::recover_secret(n, shares)
        }
        let secret_data = SecretData::with_secret("Hello World!", 5);

        let shares = vec![
            secret_data.get_share(1).unwrap(),
            secret_data.get_share(2).unwrap(),
            secret_data.get_share(3).unwrap(),
            secret_data.get_share(4).unwrap(),
            secret_data.get_share(5).unwrap(),
        ];

        let recovered = try_recover(5, &shares);
        assert!(recovered.is_some());

        let recovered = try_recover(3, &shares);
        assert!(recovered.is_none());
    }
}

pub struct SecretData {
    pub secret_data: Option<String>,
    pub coefficients: Vec<Vec<u8>>,
}

#[derive(Debug)]
pub enum ShamirError {
    /// The number of shares must be between 1 and 255
    InvalidShareCount,
}

impl SecretData {
    pub fn with_secret(secret: &str, threshold: u8) -> SecretData {
        let mut coefficients: Vec<Vec<u8>> = vec![];
        let mut rng = thread_rng();
        let mut rand_container = vec![0u8; (threshold - 1) as usize];
        for c in secret.as_bytes() {
            rng.fill_bytes(&mut rand_container);
            let mut coef: Vec<u8> = vec![*c];
            for r in rand_container.iter() {
                coef.push(*r);
            }
            coefficients.push(coef);
        }

        SecretData {
            secret_data: Some(secret.to_string()),
            coefficients,
        }
    }

    pub fn get_share(&self, id: u8) -> Result<Vec<u8>, ShamirError> {
        if id == 0 {
            return Err(ShamirError::InvalidShareCount);
        }
        let mut share_bytes: Vec<u8> = vec![];
        let coefficients = self.coefficients.clone();
        for coefficient in coefficients {
            let b = SecretData::accumulate_share_bytes(id, coefficient)?;
            share_bytes.push(b);
        }

        share_bytes.insert(0, id);
        Ok(share_bytes)
    }

    pub fn is_valid_share(&self, share: &[u8]) -> bool {
        let id = share[0];
        match self.get_share(id) {
            Ok(s) => s == share,
            _ => false,
        }
    }

    pub fn recover_secret(threshold: u8, shares: Vec<Vec<u8>>) -> Option<String> {
        if threshold as usize > shares.len() {
            println!("Number of shares is below the threshold");
            return None;
        }
        let mut xs: Vec<u8> = vec![];

        for share in shares.iter() {
            if xs.contains(&share[0]) {
                println!("Multiple shares with the same first byte");
                return None;
            }

            if share.len() != shares[0].len() {
                println!("Shares have different lengths");
                return None;
            }

            xs.push(share[0].to_owned());
        }
        let mut mycoefficients: Vec<String> = vec![];
        let mut mysecretdata: Vec<u8> = vec![];
        let rounds = shares[0].len() - 1;

        for byte_to_use in 0..rounds {
            let mut fxs: Vec<u8> = vec![];
            for share in shares.clone() {
                fxs.push(share[1..][byte_to_use]);
            }

            match SecretData::full_lagrange(&xs, &fxs) {
                None => return None,
                Some(resulting_poly) => {
                    mycoefficients.push(String::from_utf8_lossy(&resulting_poly[..]).to_string());
                    mysecretdata.push(resulting_poly[0]);
                }
            }
        }

        match String::from_utf8(mysecretdata) {
            Ok(s) => Some(s),
            Err(e) => {
                println!("{:?}", e);
                None
            }
        }
    }

    fn accumulate_share_bytes(id: u8, coefficient_bytes: Vec<u8>) -> Result<u8, ShamirError> {
        if id == 0 {
            return Err(ShamirError::InvalidShareCount);
        }
        let mut accumulator: u8 = 0;

        let mut x_i: u8 = 1;

        for c in coefficient_bytes {
            accumulator = SecretData::gf256_add(accumulator, SecretData::gf256_mul(c, x_i));
            x_i = SecretData::gf256_mul(x_i, id);
        }

        Ok(accumulator)
    }

    fn full_lagrange(xs: &[u8], fxs: &[u8]) -> Option<Vec<u8>> {
        let mut returned_coefficients: Vec<u8> = vec![];
        let len = fxs.len();
        for i in 0..len {
            let mut this_polynomial: Vec<u8> = vec![1];

            for j in 0..len {
                if i == j {
                    continue;
                }

                let denominator = SecretData::gf256_sub(xs[i], xs[j]);
                let first_term = SecretData::gf256_checked_div(xs[j], denominator);
                let second_term = SecretData::gf256_checked_div(1, denominator);
                match (first_term, second_term) {
                    (Some(a), Some(b)) => {
                        let this_term = vec![a, b];
                        this_polynomial =
                            SecretData::multiply_polynomials(&this_polynomial, &this_term);
                    }
                    (_, _) => return None,
                };
            }
            if fxs.len() + 1 >= i {
                this_polynomial = SecretData::multiply_polynomials(&this_polynomial, &[fxs[i]])
            }
            returned_coefficients =
                SecretData::add_polynomials(&returned_coefficients, &this_polynomial);
        }
        Some(returned_coefficients)
    }

    #[inline]
    fn gf256_add(a: u8, b: u8) -> u8 {
        a ^ b
    }

    #[inline]
    fn gf256_sub(a: u8, b: u8) -> u8 {
        SecretData::gf256_add(a, b)
    }

    #[inline]
    fn gf256_mul(a: u8, b: u8) -> u8 {
        if a == 0 || b == 0 {
            0
        } else {
            GF256_EXP[((u16::from(GF256_LOG[a as usize]) + u16::from(GF256_LOG[b as usize])) % 255)
                as usize]
        }
    }

    #[inline]
    fn gf256_checked_div(a: u8, b: u8) -> Option<u8> {
        if a == 0 {
            Some(0)
        } else if b == 0 {
            None
        } else {
            let a_log = i16::from(GF256_LOG[a as usize]);
            let b_log = i16::from(GF256_LOG[b as usize]);

            let mut diff = a_log - b_log;

            if diff < 0 {
                diff += 255;
            }
            Some(GF256_EXP[(diff % 255) as usize])
        }
    }

    #[inline]
    fn multiply_polynomials(a: &[u8], b: &[u8]) -> Vec<u8> {
        let mut resultterms: Vec<u8> = vec![];

        let mut termpadding: Vec<u8> = vec![];

        for bterm in b {
            let mut thisvalue = termpadding.clone();
            for aterm in a {
                thisvalue.push(SecretData::gf256_mul(*aterm, *bterm));
            }
            resultterms = SecretData::add_polynomials(&resultterms, &thisvalue);
            termpadding.push(0);
        }
        resultterms
    }

    #[inline]
    fn add_polynomials(a: &[u8], b: &[u8]) -> Vec<u8> {
        let mut a = a.to_owned();
        let mut b = b.to_owned();
        if a.len() < b.len() {
            let mut t = vec![0; b.len() - a.len()];
            a.append(&mut t);
        } else if a.len() > b.len() {
            let mut t = vec![0; a.len() - b.len()];
            b.append(&mut t);
        }
        let mut results: Vec<u8> = vec![];

        for i in 0..a.len() {
            results.push(SecretData::gf256_add(a[i], b[i]));
        }
        results
    }
}

static GF256_EXP: [u8; 256] = [
    0x01, 0x03, 0x05, 0x0f, 0x11, 0x33, 0x55, 0xff, 0x1a, 0x2e, 0x72, 0x96, 0xa1, 0xf8, 0x13, 0x35,
    0x5f, 0xe1, 0x38, 0x48, 0xd8, 0x73, 0x95, 0xa4, 0xf7, 0x02, 0x06, 0x0a, 0x1e, 0x22, 0x66, 0xaa,
    0xe5, 0x34, 0x5c, 0xe4, 0x37, 0x59, 0xeb, 0x26, 0x6a, 0xbe, 0xd9, 0x70, 0x90, 0xab, 0xe6, 0x31,
    0x53, 0xf5, 0x04, 0x0c, 0x14, 0x3c, 0x44, 0xcc, 0x4f, 0xd1, 0x68, 0xb8, 0xd3, 0x6e, 0xb2, 0xcd,
    0x4c, 0xd4, 0x67, 0xa9, 0xe0, 0x3b, 0x4d, 0xd7, 0x62, 0xa6, 0xf1, 0x08, 0x18, 0x28, 0x78, 0x88,
    0x83, 0x9e, 0xb9, 0xd0, 0x6b, 0xbd, 0xdc, 0x7f, 0x81, 0x98, 0xb3, 0xce, 0x49, 0xdb, 0x76, 0x9a,
    0xb5, 0xc4, 0x57, 0xf9, 0x10, 0x30, 0x50, 0xf0, 0x0b, 0x1d, 0x27, 0x69, 0xbb, 0xd6, 0x61, 0xa3,
    0xfe, 0x19, 0x2b, 0x7d, 0x87, 0x92, 0xad, 0xec, 0x2f, 0x71, 0x93, 0xae, 0xe9, 0x20, 0x60, 0xa0,
    0xfb, 0x16, 0x3a, 0x4e, 0xd2, 0x6d, 0xb7, 0xc2, 0x5d, 0xe7, 0x32, 0x56, 0xfa, 0x15, 0x3f, 0x41,
    0xc3, 0x5e, 0xe2, 0x3d, 0x47, 0xc9, 0x40, 0xc0, 0x5b, 0xed, 0x2c, 0x74, 0x9c, 0xbf, 0xda, 0x75,
    0x9f, 0xba, 0xd5, 0x64, 0xac, 0xef, 0x2a, 0x7e, 0x82, 0x9d, 0xbc, 0xdf, 0x7a, 0x8e, 0x89, 0x80,
    0x9b, 0xb6, 0xc1, 0x58, 0xe8, 0x23, 0x65, 0xaf, 0xea, 0x25, 0x6f, 0xb1, 0xc8, 0x43, 0xc5, 0x54,
    0xfc, 0x1f, 0x21, 0x63, 0xa5, 0xf4, 0x07, 0x09, 0x1b, 0x2d, 0x77, 0x99, 0xb0, 0xcb, 0x46, 0xca,
    0x45, 0xcf, 0x4a, 0xde, 0x79, 0x8b, 0x86, 0x91, 0xa8, 0xe3, 0x3e, 0x42, 0xc6, 0x51, 0xf3, 0x0e,
    0x12, 0x36, 0x5a, 0xee, 0x29, 0x7b, 0x8d, 0x8c, 0x8f, 0x8a, 0x85, 0x94, 0xa7, 0xf2, 0x0d, 0x17,
    0x39, 0x4b, 0xdd, 0x7c, 0x84, 0x97, 0xa2, 0xfd, 0x1c, 0x24, 0x6c, 0xb4, 0xc7, 0x52, 0xf6, 0x01,
];

static GF256_LOG: [u8; 256] = [
    0x00, 0x00, 0x19, 0x01, 0x32, 0x02, 0x1a, 0xc6, 0x4b, 0xc7, 0x1b, 0x68, 0x33, 0xee, 0xdf, 0x03,
    0x64, 0x04, 0xe0, 0x0e, 0x34, 0x8d, 0x81, 0xef, 0x4c, 0x71, 0x08, 0xc8, 0xf8, 0x69, 0x1c, 0xc1,
    0x7d, 0xc2, 0x1d, 0xb5, 0xf9, 0xb9, 0x27, 0x6a, 0x4d, 0xe4, 0xa6, 0x72, 0x9a, 0xc9, 0x09, 0x78,
    0x65, 0x2f, 0x8a, 0x05, 0x21, 0x0f, 0xe1, 0x24, 0x12, 0xf0, 0x82, 0x45, 0x35, 0x93, 0xda, 0x8e,
    0x96, 0x8f, 0xdb, 0xbd, 0x36, 0xd0, 0xce, 0x94, 0x13, 0x5c, 0xd2, 0xf1, 0x40, 0x46, 0x83, 0x38,
    0x66, 0xdd, 0xfd, 0x30, 0xbf, 0x06, 0x8b, 0x62, 0xb3, 0x25, 0xe2, 0x98, 0x22, 0x88, 0x91, 0x10,
    0x7e, 0x6e, 0x48, 0xc3, 0xa3, 0xb6, 0x1e, 0x42, 0x3a, 0x6b, 0x28, 0x54, 0xfa, 0x85, 0x3d, 0xba,
    0x2b, 0x79, 0x0a, 0x15, 0x9b, 0x9f, 0x5e, 0xca, 0x4e, 0xd4, 0xac, 0xe5, 0xf3, 0x73, 0xa7, 0x57,
    0xaf, 0x58, 0xa8, 0x50, 0xf4, 0xea, 0xd6, 0x74, 0x4f, 0xae, 0xe9, 0xd5, 0xe7, 0xe6, 0xad, 0xe8,
    0x2c, 0xd7, 0x75, 0x7a, 0xeb, 0x16, 0x0b, 0xf5, 0x59, 0xcb, 0x5f, 0xb0, 0x9c, 0xa9, 0x51, 0xa0,
    0x7f, 0x0c, 0xf6, 0x6f, 0x17, 0xc4, 0x49, 0xec, 0xd8, 0x43, 0x1f, 0x2d, 0xa4, 0x76, 0x7b, 0xb7,
    0xcc, 0xbb, 0x3e, 0x5a, 0xfb, 0x60, 0xb1, 0x86, 0x3b, 0x52, 0xa1, 0x6c, 0xaa, 0x55, 0x29, 0x9d,
    0x97, 0xb2, 0x87, 0x90, 0x61, 0xbe, 0xdc, 0xfc, 0xbc, 0x95, 0xcf, 0xcd, 0x37, 0x3f, 0x5b, 0xd1,
    0x53, 0x39, 0x84, 0x3c, 0x41, 0xa2, 0x6d, 0x47, 0x14, 0x2a, 0x9e, 0x5d, 0x56, 0xf2, 0xd3, 0xab,
    0x44, 0x11, 0x92, 0xd9, 0x23, 0x20, 0x2e, 0x89, 0xb4, 0x7c, 0xb8, 0x26, 0x77, 0x99, 0xe3, 0xa5,
    0x67, 0x4a, 0xed, 0xde, 0xc5, 0x31, 0xfe, 0x18, 0x0d, 0x63, 0x8c, 0x80, 0xc0, 0xf7, 0x70, 0x07,
];
