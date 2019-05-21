import 'package:equatable/equatable.dart';

import 'package:pixel_flutter_web/models/Topic.dart';

abstract class UpdateState extends Equatable {
  UpdateState([List props = const []]) : super(props);
}

class Updated extends UpdateState {}

class Updating extends UpdateState {}

class GotError extends UpdateState {
  final String error;

  GotError({this.error}) : super([error]);
}

class GotTopic extends UpdateState {
  final Topic topic;

  GotTopic({this.topic}) : super([topic]);
}
