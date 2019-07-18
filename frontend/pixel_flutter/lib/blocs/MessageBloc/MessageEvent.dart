import 'package:equatable/equatable.dart';
import 'package:pixel_flutter/models/Message.dart';


abstract class MessageEvent extends Equatable {
  MessageEvent([List props = const []]) : super(props);
}

class GotNew extends MessageEvent {
  final List<Message> msg;

  GotNew({this.msg}) : super([msg]);
}

class GotHistory extends MessageEvent {
  final List<Message> msg;

  GotHistory({this.msg}) : super([msg]);
}
