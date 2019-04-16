import 'package:equatable/equatable.dart';
import 'package:meta/meta.dart';

@immutable
class RegisterState extends Equatable {
  final String email;
  final bool isEmailValid;
  final bool isUsernameValid;
  final String username;
  final String password;
  final bool isPasswordValid;

  bool get isLoginValid => isUsernameValid && isPasswordValid;
  bool get isRegisterValid => isEmailValid && isPasswordValid && isUsernameValid;

  RegisterState({
    @required this.email,
    @required this.username,
    @required this.password,

    @required this.isEmailValid,
    @required this.isUsernameValid,
    @required this.isPasswordValid,
  }) : super([
    email,
    username,
    isEmailValid,
    isUsernameValid,
    password,
    isPasswordValid,
  ]);

  factory RegisterState.initial() {
    return RegisterState(
      email: '',
      username: '',
      password: '',
      isUsernameValid: false,
      isEmailValid: false,
      isPasswordValid: false,
    );
  }

  RegisterState copyWith({
    String email,
    String username,
    bool isEmailValid,
    bool isUsernameValid,
    String password,
    bool isPasswordValid,
    bool formSubmittedSuccessfully,
  }) {
    return RegisterState(
      email: email ?? this.email,
      username: username?? this.username,
      password: password ?? this.password,
      isEmailValid: isEmailValid ?? this.isEmailValid,
      isUsernameValid: isUsernameValid?? this. isUsernameValid,
      isPasswordValid: isPasswordValid ?? this.isPasswordValid,
    );
  }
}