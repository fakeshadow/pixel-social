import 'package:equatable/equatable.dart';
import 'package:pixel_flutter/models/Message.dart';

abstract class MessageState extends Equatable {
  MessageState([List props = const []]) : super(props);
}

class MessageUninitialized extends MessageState {}

class MessageLoaded extends MessageState {
  final List<Message> msg;

  MessageLoaded({
    this.msg,
  }) : super([msg]);

  MessageLoaded copyWith({
    List<Message> msg,
  }) {
    return MessageLoaded(
      msg: msg ?? this.msg,
    );
  }
}
