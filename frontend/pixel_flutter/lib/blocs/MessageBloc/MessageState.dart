import 'package:equatable/equatable.dart';

abstract class MessageState extends Equatable {
  MessageState([List props = const []]) : super(props);
}

class MessageUninitialized extends MessageState {}

class MessageLoaded extends MessageState {
  final List pubMsg;
  final List prvMsg;

  MessageLoaded({
    this.pubMsg,
    this.prvMsg,
  }) : super([pubMsg, prvMsg]);

  MessageLoaded copyWith({
    List pubMsg,
    List prvMsg,
    bool hasReachedMax,
  }) {
    return MessageLoaded(
      pubMsg: pubMsg ?? this.pubMsg,
      prvMsg: prvMsg ?? this.prvMsg,
    );
  }
}

class MessageError extends MessageState {
  final String error;

  MessageError({this.error}) : super([error]);
}
