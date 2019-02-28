import 'package:equatable/equatable.dart';

abstract class TopicEvent extends Equatable {}

class TopicAPI extends TopicEvent {
  @override
  String toString() => 'TopicAPI';
}
