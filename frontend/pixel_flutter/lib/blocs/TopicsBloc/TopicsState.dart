import 'package:equatable/equatable.dart';
import 'package:pixel_flutter/models/Topic.dart';

abstract class TopicsState extends Equatable {
  TopicsState([List props = const []]) : super(props);
}

class TopicsUninitialized extends TopicsState {}

class TopicsError extends TopicsState {}

class TopicsNone extends TopicsState {}

class TopicsLoaded extends TopicsState {
  final List<Topic> topics;
  final bool hasReachedMax;

  TopicsLoaded({
    this.topics,
    this.hasReachedMax,
  }) : super([topics, hasReachedMax]);

  TopicsLoaded copyWith({
    List<Topic> topics,
    bool hasReachedMax,
  }) {
    return TopicsLoaded(
      topics: topics ?? this.topics,
      hasReachedMax: hasReachedMax ?? this.hasReachedMax,
    );
  }
}
