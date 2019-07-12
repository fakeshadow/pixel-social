import 'package:flutter/material.dart';
import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter/blocs/TalkBloc/TalkBloc.dart';
import 'package:pixel_flutter/blocs/TalkBloc/TalkState.dart';

class TalkPage extends StatefulWidget {
  @override
  _TalkPageState createState() => _TalkPageState();
}

class _TalkPageState extends State<TalkPage> {
  @override
  Widget build(BuildContext context) {
    return BlocBuilder(
      bloc: BlocProvider.of<TalkBloc>(context),
      builder: (BuildContext context, TalkState state) {
        if (state is TalkLoaded) {
          return ListView.builder(
            itemCount: state.talks.length,
            itemBuilder: ((context, index) {
              return Text(state.talks[index].name);
            }),
          );
        }
        return Container();
      },
    );
  }
}
