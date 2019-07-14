import 'package:equatable/equatable.dart';
import 'package:meta/meta.dart';

import 'package:pixel_flutter/models/User.dart';

abstract class UserEvent extends Equatable {
  UserEvent([List props = const []]) : super(props);
}

class UserInit extends UserEvent {
  final User user;

  UserInit({this.user}): super([user]);
}

class Registering extends UserEvent {
  final String username;
  final String password;
  final String email;

  Registering(
      {@required this.username, @required this.password, @required this.email})
      : super([username, password, email]);
}

class LoggingIn extends UserEvent {
  final String username;
  final String password;

  LoggingIn({@required this.username, @required this.password})
      : super([username, password]);
}

class LoggingOut extends UserEvent {}

class Delete extends UserEvent {}