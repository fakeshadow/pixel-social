import 'package:equatable/equatable.dart';

abstract class TopicEvent extends Equatable {}

class GetTopics extends TopicEvent {
  final int categoryId;

  GetTopics(this.categoryId) ;
}
