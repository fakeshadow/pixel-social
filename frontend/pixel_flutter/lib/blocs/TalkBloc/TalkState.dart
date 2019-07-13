import 'package:equatable/equatable.dart';

abstract class TalkState extends Equatable {
  TalkState([List props = const []]) : super(props);
}

class TalkUninitialized extends TalkState {}

class TalkLoaded extends TalkState {
  final List talks;

  TalkLoaded({this.talks}) : super([talks]);

  TalkLoaded copyWith({
    List talks,
  }) {
    return TalkLoaded(
      talks: talks ?? this.talks,
    );
  }
}