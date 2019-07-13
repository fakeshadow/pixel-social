import 'package:equatable/equatable.dart';
import 'package:pixel_flutter/models/Talk.dart';

abstract class TalkEvent extends Equatable {
  TalkEvent([List props = const []]) : super(props);
}

class GotTalks extends TalkEvent {
  final List<Talk> talks;

  GotTalks({this.talks}) : super([talks]);
}

class SendMessage extends TalkEvent {
  final String msg;

  SendMessage({this.msg}) : super([msg]);
}

class GetTalk extends TalkEvent {
  final int talkId;

  GetTalk({this.talkId}) : super([talkId]);
}

class TalkInit extends TalkEvent {}
