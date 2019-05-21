import 'package:equatable/equatable.dart';
import 'package:flutter_web/widgets.dart';
import 'package:meta/meta.dart';

abstract class UserEvent extends Equatable {
  UserEvent([List props = const []]) : super(props);
}

class UserInit extends UserEvent {}

class Registering extends UserEvent {
  final String username, password, email;

  Registering(
      {@required this.username, @required this.password, @required this.email})
      : super([username, password, email]);
}

class LoggingIn extends UserEvent {
  final String username, password;

  LoggingIn({@required this.username, @required this.password})
      : super([username, password]);
}

class LoggingOut extends UserEvent {}

class Delete extends UserEvent {}

class AddTopic extends UserEvent {
  final String thumbnail, title, body;
  final int categoryId;

  AddTopic(
      {this.thumbnail,
      @required this.body,
      @required this.title,
      @required this.categoryId})
      : super([title, body, categoryId, thumbnail]);
}
