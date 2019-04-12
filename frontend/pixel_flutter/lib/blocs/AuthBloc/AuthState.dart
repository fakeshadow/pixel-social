import 'package:equatable/equatable.dart';

abstract class AuthState extends Equatable {
  AuthState([List props = const []]) : super(props);
}

class AuthUninitialized extends AuthState {}

class AuthAuthenticated extends AuthState {}