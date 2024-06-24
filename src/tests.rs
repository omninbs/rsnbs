mod tests {
    use super::super::parser::read_nbs;
    use super::super::Song;
    const TEST_FILE_SOURCE: &str = "test_files/Megalovania.nbs";
    const TEST_FILE_DEST: &str = "test_files/MegalovaniaTest.nbs";
    
    #[test]
    fn io() {
        let mut file_before_writing = read_nbs(TEST_FILE_SOURCE).expect("Some reading error occured before writing");

        file_before_writing.save(TEST_FILE_DEST, 5).expect("Some writing error ocurred");
        
        let file_after_writing = read_nbs(TEST_FILE_DEST).expect("Some reading error occured after writing");

        assert_eq!(file_before_writing, file_after_writing);
    }
}
