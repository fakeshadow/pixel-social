import 'package:equatable/equatable.dart';

import 'package:pixel_flutter/models/User.dart';

abstract class UsersEvent extends Equatable {
  UsersEvent([List props = const []]) : super(props);
}

class GotUsers extends UsersEvent {
  final List<User> users;

  GotUsers({this.users}) : super([users]);
}

class GotRelations extends UsersEvent {
  final List<int> friends;

  GotRelations({this.friends}) : super([friends]);
}