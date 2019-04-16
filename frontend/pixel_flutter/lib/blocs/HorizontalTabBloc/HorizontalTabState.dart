import 'package:equatable/equatable.dart';

abstract class HorizontalTabState extends Equatable {
  HorizontalTabState([List props = const []]) : super(props);
}

class Selected extends HorizontalTabState {

  final int index;
  Selected({this.index}):super([index]);
}
