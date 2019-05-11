import 'package:equatable/equatable.dart';
import 'package:pixel_flutter_web/models/Topic.dart';

abstract class TopicState extends Equatable {
  TopicState([List props = const []]) : super(props);
}

class TopicUninitialized extends TopicState {}

class TopicError extends TopicState {}

class TopicLoaded extends TopicState {
  final List<Topic> topics;
  final bool hasReachedMax;

  TopicLoaded({
    this.topics,
    this.hasReachedMax,
  }) : super([topics, hasReachedMax]);

  TopicLoaded copyWith({
    List<Topic> topics,
    bool hasReachedMax,
  }) {
    return TopicLoaded(
      topics: topics ?? this.topics,
      hasReachedMax: hasReachedMax ?? this.hasReachedMax,
    );
  }
}
