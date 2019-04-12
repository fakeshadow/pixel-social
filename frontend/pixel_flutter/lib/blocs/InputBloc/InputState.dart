import 'package:equatable/equatable.dart';
import 'package:meta/meta.dart';

@immutable
class InputState extends Equatable {
  final String email;
  final bool isEmailValid;
  final bool isUsernameValid;
  final String username;
  final String password;
  final bool isPasswordValid;
  final bool formSubmittedSuccessfully;

  bool get isFormValid => isEmailValid && isPasswordValid && isUsernameValid;

  InputState({
    @required this.email,
    @required this.username,
    @required this.isEmailValid,
    @required this.isUsernameValid,
    @required this.password,
    @required this.isPasswordValid,
    @required this.formSubmittedSuccessfully,
  }) : super([
    email,
    username,
    isEmailValid,
    isUsernameValid,
    password,
    isPasswordValid,
    formSubmittedSuccessfully,
  ]);

  factory InputState.initial() {
    return InputState(
      email: '',
      username: "",
      isUsernameValid: false,
      isEmailValid: false,
      password: '',
      isPasswordValid: false,
      formSubmittedSuccessfully: false,
    );
  }



  InputState copyWith({
    String email,
    String username,
    bool isEmailValid,
    bool isUsernameValid,
    String password,
    bool isPasswordValid,
    bool formSubmittedSuccessfully,
  }) {
    return InputState(
      email: email ?? this.email,
      isEmailValid: isEmailValid ?? this.isEmailValid,
      isUsernameValid: isUsernameValid?? this. isUsernameValid,
      username: username?? this.username,
      password: password ?? this.password,
      isPasswordValid: isPasswordValid ?? this.isPasswordValid,
      formSubmittedSuccessfully:
      formSubmittedSuccessfully ?? this.formSubmittedSuccessfully,
    );
  }
}