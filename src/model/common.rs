pub trait GetSelfField {
    fn get_self_id(&self) -> &i32;
}

pub trait MatchUser {
    fn get_user_id(&self) -> &i32;

    fn match_user_index<T>(&self, users: &Vec<T>) -> Option<usize>
        where T: GetSelfField {
        let mut _index: Vec<usize> = Vec::with_capacity(1);
        for (index, user) in users.iter().enumerate() {
            if &self.get_user_id() == &user.get_self_id() {
                _index.push(index);
                break;
            }
        };
        if _index.len() == 0 { return None; }
        Some(_index[0])
    }

    // add user privacy filter here
    fn make_user_field<T>(&self, users: &Vec<T>) -> Option<T>
        where T: Clone + GetSelfField {
        match &self.match_user_index(users) {
            Some(index) => Some(users[*index].clone()),
            None => None
        }
    }
}

// need to improve validator with regex
pub trait Validator {
    fn get_username(&self) -> &str;
    fn get_password(&self) -> &str;
    fn get_email(&self) -> &str;

    fn check_username(&self) -> bool {
        let username = self.get_username();
        if username.len() < 3 { return false; };
        true
    }

    fn check_password(&self) -> bool {
        let password = self.get_password();
        if password.len() < 8 { return false; };
        true
    }

    fn check_email(&self) -> bool {
        let email = self.get_email();
        if !email.contains("@") { return false; }
        let vec: Vec<&str> = email.split("@").collect();
        if vec[0].len() < 3 || vec[1].len() < 3 { return false; }
        true
    }

    fn check_register(&self) -> bool {
        if !&self.check_email() || !&self.check_password() || !&self.check_username() {
            return false
        }
        true
    }
}