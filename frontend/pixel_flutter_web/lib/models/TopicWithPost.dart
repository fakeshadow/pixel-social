import 'package:equatable/equatable.dart';
import './Post.dart';
import './Topic.dart';

class TopicWithPost extends Equatable {
  final Topic topic;
  final List<Post> posts;

  TopicWithPost({this.topic, this.posts}) : super([topic, posts]);
}
