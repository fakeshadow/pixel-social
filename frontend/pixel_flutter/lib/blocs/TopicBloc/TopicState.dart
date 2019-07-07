import 'package:equatable/equatable.dart';
import 'package:pixel_flutter/models/Topic.dart';

abstract class TopicState extends Equatable {
  TopicState([List props = const []]) : super(props);
}

class TopicUninitialized extends TopicState {}

class TopicLoaded extends TopicState {
  final Topic topic;
  final List posts;
  final bool hasReachedMax;

  TopicLoaded({
    this.topic,
    this.posts,
    this.hasReachedMax,
  }) : super([topic, posts, hasReachedMax]);

  TopicLoaded copyWith({
    Topic topic,
    List posts,
    bool hasReachedMax,
  }) {
    return TopicLoaded(
      topic: topic ?? this.topic,
      posts: posts ?? this.posts,
      hasReachedMax: hasReachedMax ?? this.hasReachedMax,
    );
  }
}

class GotError extends TopicState {
  final String error;

  GotError({this.error}) : super([error]);
}