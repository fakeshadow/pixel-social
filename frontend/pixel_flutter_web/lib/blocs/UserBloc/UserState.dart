import 'package:flutter_web/widgets.dart';
import 'package:meta/meta.dart';
import 'package:equatable/equatable.dart';
import 'package:pixel_flutter_web/models/User.dart';

abstract class UserState extends Equatable {
  UserState([List props = const []]) : super(props);
}

class Loading extends UserState {}

class UserLoaded extends UserState {
  final User user;

  UserLoaded({@required this.user}) : super([user]);
}

class UserNone extends UserState {}

class UserLoggedOut extends UserState {
  final String username;

  UserLoggedOut({@required this.username}):super([username]);
}

class Failure extends UserState {
  final String error;

  Failure({@required this.error}) : super([error]);
}
