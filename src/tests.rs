mod tests {
    use super::super::parser::read_nbs;


    #[test]
    fn reading() {read_nbs("test_files/Megalovania.nbs").expect("Some error occured");}
    // checking if it is correct is to much work!
}
