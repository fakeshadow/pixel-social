import 'package:flutter_web/material.dart';

import 'package:flutter_bloc/flutter_bloc.dart';

import 'package:pixel_flutter_web/env.dart';
import 'package:pixel_flutter_web/blocs/TopicsBlocs.dart';

import 'package:pixel_flutter_web/components/SideMenu.dart';
import 'package:pixel_flutter_web/components/TopicTile.dart';
import 'package:pixel_flutter_web/components/BottomLoader.dart';
import 'package:pixel_flutter_web/views/TopicPage.dart';


class TopicsLayout extends StatelessWidget with env{
  final TopicsBloc topicBloc;
  final ScrollController controller;

  TopicsLayout({this.topicBloc, this.controller});

  @override
  Widget build(BuildContext context) {
    return Row(mainAxisAlignment: MainAxisAlignment.center, children: <Widget>[
      Container(
        width: MediaQuery.of(context).size.width > 700
            ? 700
            : MediaQuery.of(context).size.width,
        child: BlocBuilder(
            bloc: topicBloc,
            builder: (context, state) {
              if (state is TopicsLoaded) {
                return ListView.builder(
                    controller: controller,
                    itemCount: state.hasReachedMax
                        ? state.topics.length
                        : state.topics.length + 1,
                    itemBuilder: (context, index) {
                      return index >= state.topics.length &&
                          !state.hasReachedMax
                          ? BottomLoader()
                          : TopicTile(
                          topic: state.topics[index],
                          onTap: () => pushToTopicPage(context, state.topics[index]));
                    });
              } else {
                return Container();
              }
            }),
      ),
      MediaQuery.of(context).size.width > BREAK_POINT_WIDTH
          ? SideMenu()
          : Container()
    ]);
  }

  pushToTopicPage(context, topic) {
    Navigator.push(context,
        MaterialPageRoute(builder: (context) => TopicPage(topic: topic)));
  }
}