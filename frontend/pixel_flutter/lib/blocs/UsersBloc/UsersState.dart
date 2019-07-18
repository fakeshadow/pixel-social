import 'package:flutter/widgets.dart';
import 'package:meta/meta.dart';
import 'package:equatable/equatable.dart';
import 'package:pixel_flutter/models/User.dart';

@immutable
class UsersState extends Equatable {
  final List<User> users;
  final List<int> friends;

  UsersState({
    @required this.users,
    @required this.friends,
  }) : super([
    users,
    friends,
  ]);

  factory UsersState.initial() {
    return UsersState(
      users: [],
      friends: [],
    );
  }

  UsersState copyWith({
    List<User> users,
    List<int> friends,
  }) {
    return UsersState(
      users: users ?? this.users,
      friends: friends ?? this.friends,
    );
  }
}