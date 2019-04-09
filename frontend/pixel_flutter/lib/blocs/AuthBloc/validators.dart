import 'dart:async';

const MIN_USERNAME = 5;
const MIN_EMAIL = 3;
const MIN_PASSWORD = 6;

class Validators {
  final validateEmail =
      StreamTransformer<String, String>.fromHandlers(handleData: (email, sink) {
    if (email.contains('@')) {
      final vec = email.split('@');
      if (vec.length != 2 ||
          vec[0].length < MIN_EMAIL ||
          vec[1].length < MIN_EMAIL) {
        sink.addError('Please use a valid email address');
      } else {
        sink.add(email);
      }
    } else {
      sink.addError('Please use a valid email address');
    }
  });
  final validateUsername = StreamTransformer<String, String>.fromHandlers(
      handleData: (username, sink) {
    if (username.length < MIN_USERNAME) {
      sink.addError('Username has to be at lease 5 chars long');
    } else {
      sink.add(username);
    }
  });
  final validatePassword = StreamTransformer<String, String>.fromHandlers(
      handleData: (password, sink) {
    if (password.length < MIN_PASSWORD) {
      sink.addError('Password has to be at lease 6 chars long');
    } else {
      sink.add(password);
    }
  });
}
