import 'package:equatable/equatable.dart';
import 'package:meta/meta.dart';

abstract class TopicEvent extends Equatable {
  TopicEvent([List props = const[]]) : super(props);
}

class GetTopics extends TopicEvent {
  final int categoryId;

  GetTopics({@required this.categoryId}) : super([categoryId]);
}

class GetMore extends TopicEvent {}