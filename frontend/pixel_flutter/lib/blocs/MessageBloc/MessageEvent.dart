import 'package:equatable/equatable.dart';
import 'package:pixel_flutter/models/Message.dart';


abstract class MessageEvent extends Equatable {
  MessageEvent([List props = const []]) : super(props);
}

class Init extends MessageEvent {

}

class GotMessage extends MessageEvent {
  final List<Message> msg;

  GotMessage({this.msg}) : super([msg]);
}
