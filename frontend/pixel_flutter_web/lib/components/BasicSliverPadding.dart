import 'package:flutter_web/material.dart';

import 'package:pixel_flutter_web/env.dart';

class BasicSliverPadding extends StatelessWidget with env {
  final Widget sliver;

  BasicSliverPadding({this.sliver});

  @override
  Widget build(BuildContext context) {
    return SliverPadding(
        padding: EdgeInsets.only(
          left: MediaQuery.of(context).size.width > BREAK_POINT_WIDTH
              ? MediaQuery.of(context).size.width * 0.2
              : 0,
          right: MediaQuery.of(context).size.width > BREAK_POINT_WIDTH_SM
              ? MediaQuery.of(context).size.width * 0.4
              : 0,
        ),
        sliver: sliver);
  }
}
