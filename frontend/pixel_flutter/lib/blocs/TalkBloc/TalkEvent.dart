import 'package:equatable/equatable.dart';
import 'package:pixel_flutter/models/Talk.dart';

abstract class TalkEvent extends Equatable {
  TalkEvent([List props = const []]) : super(props);
}

class GotError extends TalkEvent {
  final String error;

  GotError({this.error}) : super([error]);
}

class GotTalk extends TalkEvent {
  final List<Talk> talks;

  GotTalk({this.talks}) : super([talks]);
}
