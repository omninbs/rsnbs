mod tests {
    use super::super::parser::read_nbs;
    use super::super::Song;
    use diff::chars;
    const TEST_FILES: [&str;2] = ["test_files/Megalovania.nbs", "test_files/DB.nbs"];

    #[test]
    fn io() { for test_file in TEST_FILES {
        let mut file_before_writing = read_nbs(test_file).expect("Some reading error occured before writing");

        let test_file_dest = (test_file.to_string() + "Test");

        file_before_writing.save(test_file_dest.as_str(), file_before_writing.header.version.unwrap() as u8).expect("Some writing error ocurred");
        
        let file_after_writing = read_nbs(test_file_dest.as_str()).expect("Some reading error occured after writing");

        if file_before_writing != file_after_writing {
            let dbg_before = format!("{:?}", file_before_writing);
            let dbg_after = format!("{:?}", file_after_writing);
            for diff in chars(dbg_before.as_str(), dbg_after.as_str()) {
                match diff {
                    diff::Result::Left(l) => println!("\n\n- {}\n\n", l),
                    diff::Result::Right(r) => println!("\n\n+ {}\n\n", r),
                    diff::Result::Both(b, _) => print!("{}", b),
                }
            } panic!("{} failed!", test_file);
        }
    }}
}
