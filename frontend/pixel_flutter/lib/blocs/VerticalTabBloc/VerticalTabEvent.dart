import 'package:equatable/equatable.dart';

class VerticalTabEvent extends Equatable {
  VerticalTabEvent([List props = const []]) : super(props);
}

class Tapped extends VerticalTabEvent {
  final int index;
  Tapped({this.index}):super([index]);
}

