import 'dart:async';
import 'validators.dart';
import 'package:rxdart/rxdart.dart';

class AuthBloc extends Object with Validators {
  final _usernameController = BehaviorSubject<String>();
  final _emailController = BehaviorSubject<String>();
  final _passwordController = BehaviorSubject<String>();

  // Add data to stream
  Stream<String> get username =>
      _usernameController.transform(validateUsername);

  Stream<String> get email => _emailController.stream.transform(validateEmail);

  Stream<String> get password =>
      _passwordController.stream.transform(validatePassword);

  Stream<bool> get registerValid =>
      Observable.combineLatest3(username, email, password, (u, e, p) => true);

  // Mutate data
  Function(String) get changeUsername => _usernameController.sink.add;

  Function(String) get changeEmail => _emailController.sink.add;

  Function(String) get changePassword => _passwordController.sink.add;

  dispose() {
    _usernameController.close();
    _passwordController.close();
    _emailController.close();
  }

  submit() {
    print("submited");
  }
}
