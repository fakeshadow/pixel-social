import 'package:equatable/equatable.dart';
import 'package:meta/meta.dart';

@immutable
abstract class InputEvent extends Equatable {
  InputEvent([List props = const []]) : super(props);
}

class UsernameChanged extends InputEvent {
  final String username;

  UsernameChanged({@required this.username}) : super([username]);
}

class EmailChanged extends InputEvent {
  final String email;

  EmailChanged({@required this.email}) : super([email]);
}

class PasswordChanged extends InputEvent {
  final String password;

  PasswordChanged({@required this.password}) : super([password]);
}

class FormSubmitted extends InputEvent {}

class FormReset extends InputEvent {}
