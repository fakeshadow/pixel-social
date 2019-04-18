import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';
import 'package:pixel_flutter/blocs/HorizontalTabBlocs.dart';

import 'package:pixel_flutter/style/text.dart';

class HorizontalTabText extends StatelessWidget {
  final String text;
  final int index;

  HorizontalTabText({@required this.text, @required this.index});

  @override
  Widget build(BuildContext context) {
    final _bloc = BlocProvider.of<HorizontalTabBloc>(context);
    return Transform.rotate(
        angle: -1.58,
        child: BlocBuilder(
          bloc: _bloc,
          builder: (BuildContext context, HorizontalTabState state) {
            if (state is Selected) {
              return InkWell(
                onTap: () => _bloc.dispatch(Tapped(index: index)),
                child: AnimatedDefaultTextStyle(
                  style: state.index == index
                      ? horizontalTabSelectedStyle
                      : horizontalTabStyle,
                  duration: Duration(milliseconds: 200),
                  child: Text(text),
                ),
              );
            }
          },
        ));
  }
}
