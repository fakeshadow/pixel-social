import 'package:equatable/equatable.dart';

abstract class MessageState extends Equatable {
  MessageState([List props = const []]) : super(props);
}

class MessageUninitialized extends MessageState {}

class MessageLoaded extends MessageState {
  final List msg;

  MessageLoaded({
    this.msg,
  }) : super([msg]);

  MessageLoaded copyWith({
    List pubMsg,
    List prvMsg,
    bool hasReachedMax,
  }) {
    return MessageLoaded(
      msg: pubMsg ?? this.msg,
    );
  }
}
