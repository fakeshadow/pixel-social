import 'package:equatable/equatable.dart';

class Category extends Equatable {
  final int id, topicCount, postCount, subCount;
  final String name, thumbnail;

  Category(
      {this.id,
      this.name,
      this.thumbnail,
      this.postCount,
      this.subCount,
      this.topicCount})
      : super([id, name, thumbnail, postCount, subCount, topicCount]);

  Map<String, dynamic> toMap() {
    final map = <String, dynamic>{
      'id': id,
      'name': name,
      'thumbnail': thumbnail,
      'postCount': postCount,
      'subCount': subCount,
      'topicCount': topicCount,
    };
    return map;
  }
}
