import 'package:equatable/equatable.dart';
import 'package:meta/meta.dart';

abstract class TopicsEvent extends Equatable {
  TopicsEvent([List props = const[]]) : super(props);
}

class GetTopics extends TopicsEvent {
  final int categoryId;

  GetTopics({@required this.categoryId}) : super([categoryId]);
}