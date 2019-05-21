import 'package:equatable/equatable.dart';
import 'package:meta/meta.dart';

abstract class UpdateEvent extends Equatable {
  UpdateEvent([List props = const []]) : super(props);
}

class AddTopic extends UpdateEvent {
  final String thumbnail, title, body;
  final int categoryId;

  AddTopic(
      {this.thumbnail,
      @required this.body,
      @required this.title,
      @required this.categoryId})
      : super([title, body, categoryId, thumbnail]);
}
