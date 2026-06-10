#[cfg(test)]
mod tests {
    use fountain_raptor_q::generators::degree_gen;

    #[test]
    fn test_degree_gen_sample() {
        // Test with sample k value
        // Note: These tests will work once RFC 6330 table is populated

        // Example test case (adjust based on actual parameters)
        let k = 100;
        let v = 512; // sample v value

        let degree = degree_gen(v, k);

        // Degree should be reasonable and bounded
        assert!(degree > 0, "Degree should be positive");
        assert!(degree < 100, "Degree should be reasonable");
    }
}
