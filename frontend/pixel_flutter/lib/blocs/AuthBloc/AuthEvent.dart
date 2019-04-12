import 'package:equatable/equatable.dart';

abstract class AuthEvent extends Equatable {
  AuthEvent([List props = const []]) : super(props);
}

class LoggedIn extends AuthEvent{}

class LoggedOut extends AuthEvent {}
