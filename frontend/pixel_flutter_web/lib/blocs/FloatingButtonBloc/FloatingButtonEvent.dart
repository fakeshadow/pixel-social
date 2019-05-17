import 'package:equatable/equatable.dart';
import 'package:flutter_web/widgets.dart';

abstract class FloatingButtonEvent extends Equatable {
  FloatingButtonEvent([List props = const []]) : super(props);
}

class ShowFloating extends FloatingButtonEvent {
  final bool showFloating;

  ShowFloating({@required this.showFloating}) : super([showFloating]);
}
