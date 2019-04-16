import 'package:flutter/material.dart';
import 'package:pixel_flutter/style/colors.dart';

class GeneralBackground extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return LayoutBuilder(
      builder: (context, constraint) {
        final _height = constraint.maxHeight;
        final _width = constraint.maxWidth;
        return Stack(
          children: <Widget>[
            Container(
              color: backgroundColor,
            ),
            Positioned(
              left: (_width - _height) / 2,
              bottom: _height * 0.15,
              child: Container(
                height: _height,
                width: _height,
                decoration: BoxDecoration(
                    shape: BoxShape.circle, color: firstCircleColor),
              ),
            ),
            Positioned(
              left: _width * 0.15,
              top: -_width * 0.5,
              child: Container(
                height: _width * 1.3,
                width: _width * 1.3,
                decoration: BoxDecoration(
                    shape: BoxShape.circle, color: secondCircleColor),
              ),
            ),
            Positioned(
              right: -_width * 0.1,
              top: -_width * 0.1,
              child: Container(
                height: _width * 0.4,
                width: _width * 0.4,
                decoration: BoxDecoration(
                    shape: BoxShape.circle, color: thirdCircleColor),
              ),
            )
          ],
        );
      },
    );
  }
}
