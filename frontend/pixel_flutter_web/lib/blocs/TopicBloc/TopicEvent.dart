import 'package:equatable/equatable.dart';
import 'package:meta/meta.dart';

abstract class TopicEvent extends Equatable {
  TopicEvent([List props = const[]]) : super(props);
}

class GetTopic extends TopicEvent {
  final int topicId;

  GetTopic({@required this.topicId}) : super([topicId]);
}
