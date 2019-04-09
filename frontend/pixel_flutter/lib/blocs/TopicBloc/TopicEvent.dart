import 'package:equatable/equatable.dart';

abstract class TopicEvent extends Equatable {}

class GetTopics extends TopicEvent {
  @override
  String toString() => 'TopicAPI';
}
