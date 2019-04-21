import 'package:equatable/equatable.dart';

abstract class VerticalTabState extends Equatable {
  VerticalTabState([List props = const []]) : super(props);
}

class Selected extends VerticalTabState {

  final int index;
  Selected({this.index}):super([index]);
}
