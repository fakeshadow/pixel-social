import 'package:flutter/widgets.dart';
import 'package:meta/meta.dart';
import 'package:equatable/equatable.dart';
import 'package:pixel_flutter/models/User.dart';

abstract class UserState extends Equatable {
  UserState([List props = const []]) : super(props);
}

class AppStarted extends UserState {}

class Loading extends UserState {}

class UserLoaded extends UserState {
  final User user;

  UserLoaded({@required this.user}) : super([user]);
}

class UserNone extends UserState {}

class Failure extends UserState {
  final String error;

  Failure({@required this.error}) : super([error]);
}
