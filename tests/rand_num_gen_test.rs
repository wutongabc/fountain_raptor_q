use fountain_raptor_q::generators::rand;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rand_basic() {
        // Test with basic values
        let result = rand(100, 5, 1000);
        assert!(result < 1000);
    }

    #[test]
    fn test_rand_deterministic() {
        // Same inputs should produce same output
        let result1 = rand(12345, 67, 1000);
        let result2 = rand(12345, 67, 1000);
        assert_eq!(result1, result2);
    }

    #[test]
    fn test_rand_different_inputs() {
        // Different inputs should produce different outputs (usually)
        let result1 = rand(100, 5, 1000);
        let result2 = rand(200, 5, 1000);
        // Note: this might occasionally fail due to hash collisions, but very unlikely
        assert_ne!(result1, result2);
    }

    #[test]
    fn test_rand_modulo() {
        // Result should always be less than m
        for i in 0..10 {
            let result = rand(i * 1000, i as u8, 100);
            assert!(result < 100);
        }
    }
}
