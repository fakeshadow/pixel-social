import 'package:equatable/equatable.dart';
import 'package:pixel_flutter/models/Message.dart';


abstract class MessageEvent extends Equatable {
  MessageEvent([List props = const []]) : super(props);
}

class GotError extends MessageEvent {
  final String error;

  GotError({this.error}) : super([error]);
}

class GotPublicMessage extends MessageEvent {
  final List<PublicMessage> msg;

  GotPublicMessage({this.msg}) : super([msg]);
}

class GotPrivateMessage extends MessageEvent {
  final List<PrivateMessage> msg;

  GotPrivateMessage({this.msg}) : super([msg]);
}