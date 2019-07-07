import 'package:equatable/equatable.dart';
import './Topic.dart';

class TopicWithPost extends Equatable {
  final Topic topic;
  final List posts;

  TopicWithPost({this.topic, this.posts}) : super([topic, posts]);
}
